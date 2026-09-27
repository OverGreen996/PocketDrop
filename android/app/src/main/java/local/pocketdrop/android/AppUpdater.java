package local.pocketdrop.android;

import android.app.*;
import android.content.*;
import android.content.pm.*;
import android.net.Uri;
import android.os.*;
import android.provider.Settings;
import android.widget.Toast;
import androidx.core.content.FileProvider;
import okhttp3.*;
import org.json.JSONObject;
import java.io.*;
import java.util.*;
import java.util.concurrent.*;

/** GitHub supplies only program updates; no Room data is sent to it. */
final class AppUpdater {
    private static final String MANIFEST="https://raw.githubusercontent.com/OverGreen996/PocketDrop/main/updates/android.json";
    interface BeforeInstall { void run() throws Exception; }
    private final Activity activity;
    private final BeforeInstall beforeInstall;
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private final Handler ui=new Handler(Looper.getMainLooper());
    private final OkHttpClient http=new OkHttpClient.Builder().connectTimeout(12,TimeUnit.SECONDS).readTimeout(30,TimeUnit.SECONDS).callTimeout(3,TimeUnit.MINUTES).followSslRedirects(false).build();
    private volatile Call call;
    private volatile boolean closed,cancelled;
    private boolean busy,waitingPermission,readyPrompt;
    private File apk;
    private UpdateRelease offer,ready;
    private AlertDialog dialog;
    AppUpdater(Activity a,BeforeInstall before){activity=a;beforeInstall=before;}
    private void post(Runnable r){ui.post(()->{if(!closed&&!activity.isDestroyed())r.run();});}
    private void message(String s){Toast.makeText(activity,s,Toast.LENGTH_LONG).show();}
    void resume(){
        if(waitingPermission){waitingPermission=false;if(activity.getPackageManager().canRequestPackageInstalls())install();else message("未允許安裝更新，可在「裝置」再次檢查更新。");}
        else if(ready!=null)showReady();else if(offer!=null)showOffer();
    }
    void check(){
        if(busy){message("正在檢查或下載更新");return;}
        if(ready!=null){readyPrompt=true;showReady();return;}
        busy=true;
        worker.execute(()->{try{
            call=http.newCall(new Request.Builder().url(MANIFEST).header("Cache-Control","no-cache").build());
            UpdateRelease release;
            try(Response response=call.execute()){
                if(!response.isSuccessful()||response.body()==null)throw new IOException();
                ByteArrayOutputStream data=new ByteArrayOutputStream();try(InputStream in=response.body().byteStream()){
                    byte[] buffer=new byte[4096];int n;while((n=in.read(buffer))!=-1){if(data.size()+n>32768)throw new IOException();data.write(buffer,0,n);}
                }
                release=new UpdateRelease(new JSONObject(data.toString("UTF-8")));
            }
            long current=version(activity.getPackageManager().getPackageInfo(activity.getPackageName(),0));
            post(()->{busy=false;if(release.newer(current,Build.VERSION.SDK_INT)){offer=release;showOffer();}else message("目前已是此裝置可用的最新版本");});
        }catch(Exception e){post(()->{busy=false;message("暫時無法檢查更新，LAN 分享仍可使用。可到「裝置」重試。");});}});
    }
    private void showOffer(){
        if(offer==null||!activity.hasWindowFocus()||(dialog!=null&&dialog.isShowing()))return;
        UpdateRelease release=offer;offer=null;
        dialog=new AlertDialog.Builder(activity).setTitle("PocketDrop "+release.version+" 可更新")
            .setMessage(release.notes+"\n\n按更新後下載 APK，再由 Android 確認安裝。配對與資料保留，請先完成檔案傳輸。")
            .setNegativeButton("稍後",(d,w)->offer=null).setPositiveButton("更新",(d,w)->{offer=null;download(release);}).create();
        dialog.setOnCancelListener(d->offer=null);dialog.show();
    }
    private void download(UpdateRelease release){
        try{beforeInstall.run();}catch(Exception e){message(e.getMessage());return;}
        busy=true;cancelled=false;
        dialog=new AlertDialog.Builder(activity).setTitle("下載更新").setMessage("準備下載…").setNegativeButton("取消",(d,w)->{cancelled=true;Call c=call;if(c!=null)c.cancel();}).create();
        dialog.setCancelable(false);dialog.show();
        worker.execute(()->{File target=null;try{
            File directory=new File(activity.getCacheDir(),"updates");if(!directory.exists()&&!directory.mkdirs())throw new IOException();
            target=new File(directory,"update.apk");
            call=http.newCall(new Request.Builder().url(release.url).build());
            try(Response response=call.execute()){
                if(!response.isSuccessful()||response.body()==null)throw new IOException();
                try(InputStream in=response.body().byteStream();OutputStream out=new FileOutputStream(target)){
                    final int[] last={-1};
                    release.copyVerified(in,out,(done,total)->{if(cancelled||closed)throw new IOException();int percent=(int)(done*100/total);if(percent!=last[0]){last[0]=percent;post(()->dialog.setMessage("下載中 "+percent+"%"));}});
                }
            }
            validateApk(target,release);
            File finished=target;
            post(()->{busy=false;dialog.dismiss();apk=finished;ready=release;readyPrompt=true;showReady();});
        }catch(Exception e){if(target!=null)target.delete();post(()->{busy=false;dialog.dismiss();message(cancelled?"已取消更新":"更新下載或驗證失敗，未安裝。請稍後重試。");});}});
    }
    private void showReady(){
        if(ready==null||!readyPrompt||!activity.hasWindowFocus()||(dialog!=null&&dialog.isShowing()))return;
        readyPrompt=false;
        dialog=new AlertDialog.Builder(activity).setTitle("更新已下載並驗證").setMessage("準備更新至 "+ready.version+"。安裝後保留配對與資料，Android 仍會要求你確認安裝。")
            .setNegativeButton("稍後",null).setPositiveButton("繼續安裝",(d,w)->install()).create();dialog.show();
    }
    private void install(){
        if(ready==null||apk==null)return;
        try{
            beforeInstall.run();validateApk(apk,ready);
            if(!activity.getPackageManager().canRequestPackageInstalls()){
                dialog=new AlertDialog.Builder(activity).setTitle("允許 PocketDrop 安裝更新")
                    .setMessage("Android 需要你允許這個來源安裝 App。設定完成返回後，會繼續顯示系統安裝確認。")
                    .setNegativeButton("取消",null).setPositiveButton("前往設定",(d,w)->{
                        try{waitingPermission=true;activity.startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,Uri.parse("package:"+activity.getPackageName())));}catch(Exception e){waitingPermission=false;message("無法開啟安裝權限設定");}
                    }).show();return;
            }
            Uri uri=FileProvider.getUriForFile(activity,activity.getPackageName()+".files",apk);
            Intent intent=new Intent(Intent.ACTION_VIEW).setDataAndType(uri,"application/vnd.android.package-archive").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            activity.startActivity(intent);ready=null;
        }catch(Exception e){message("無法安裝更新：請先完成傳輸，並確認系統允許安裝。");}
    }
    @SuppressWarnings("deprecation")
    private void validateApk(File file,UpdateRelease release)throws Exception{
        PackageManager pm=activity.getPackageManager();int flags=Build.VERSION.SDK_INT>=28?PackageManager.GET_SIGNING_CERTIFICATES:PackageManager.GET_SIGNATURES;
        PackageInfo archive=pm.getPackageArchiveInfo(file.getAbsolutePath(),flags),installed=pm.getPackageInfo(activity.getPackageName(),flags);
        if(archive==null||!activity.getPackageName().equals(archive.packageName)||version(archive)!=release.code||version(archive)<=version(installed)||!release.version.equals(archive.versionName))throw new IOException("更新版本不符");
        android.content.pm.Signature[] a=signers(archive),b=signers(installed);
        if(a==null||b==null||a.length==0||!new HashSet<>(Arrays.asList(a)).equals(new HashSet<>(Arrays.asList(b))))throw new IOException("更新簽章不符");
    }
    @SuppressWarnings("deprecation") private static android.content.pm.Signature[] signers(PackageInfo info){return Build.VERSION.SDK_INT>=28?(info.signingInfo==null?null:info.signingInfo.getApkContentsSigners()):info.signatures;}
    @SuppressWarnings("deprecation") private static long version(PackageInfo info){return Build.VERSION.SDK_INT>=28?info.getLongVersionCode():info.versionCode;}
    void close(){closed=true;Call c=call;if(c!=null)c.cancel();worker.shutdownNow();if(dialog!=null)dialog.dismiss();}
}
