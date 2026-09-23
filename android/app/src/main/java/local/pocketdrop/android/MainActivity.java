package local.pocketdrop.android;

import android.app.*;
import android.content.*;
import android.database.Cursor;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.*;
import android.provider.MediaStore;
import android.provider.OpenableColumns;
import android.text.*;
import android.content.ClipboardManager;
import android.view.*;
import android.widget.*;
import androidx.core.content.FileProvider;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;
import com.google.zxing.integration.android.IntentIntegrator;
import com.google.zxing.integration.android.IntentResult;
import org.json.*;
import okhttp3.*;
import okio.BufferedSink;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.*;

/** Foreground Android companion for the explicitly PC-hosted trial. */
public final class MainActivity extends Activity {
    private final Handler ui=new Handler(Looper.getMainLooper());
    private final ExecutorService network=Executors.newSingleThreadExecutor();
    private final ExecutorService transfers=Executors.newSingleThreadExecutor();
    private final ExecutorService polls=Executors.newSingleThreadExecutor();
    private final ExecutorService probes=Executors.newFixedThreadPool(2);
    private final HashSet<String> probing=new HashSet<>();
    private final EndpointCandidates candidates=new EndpointCandidates();
    private Call stateRequest;
    private long connectionEpoch;
    private long pairingAttempt;
    private Vault vault; private Nearby nearby;
    private volatile RoomClient client; private WebSocket socket;
    private boolean foreground,online,editing,updating,transferBusy;
    private volatile boolean cancelled;
    private volatile Call transfer;
    private int tab=0, retryTicks;
    private long revision=-1;
    private JSONArray files=new JSONArray();
    private LinearLayout root,content,filesView,devicesView,textView;
    private TextView status,notice,progress;
    private EditText editor;
    private Button cancel;
    private String pendingText;
    private final ArrayList<Uri> pendingUris=new ArrayList<>();
    private static final int PICK=31, INK=0xffedf5f5, MUTED=0xffabc1c9, ACCENT=0xff86ddcc;
    private final Runnable pulse=new Runnable(){public void run(){if(!foreground)return;if(client!=null){refresh();connectSocket();if(!online){retryCandidates();if(++retryTicks%6==0)discover();}}ui.postDelayed(this,5000);}};

    @Override public void onCreate(Bundle saved){super.onCreate(saved);vault=new Vault(this);nearby=new Nearby(this);buildUi();
        try{JSONObject p=vault.load();if(p!=null)client=new RoomClient(p);}catch(Exception e){notice("無法讀取已保存的配對，請重新掃描");}
        if(saved==null)receive(getIntent());renderDevices();showTab(0);
    }
    @Override protected void onStart(){super.onStart();foreground=true;restartConnection();if(client!=null)consumePending();}
    @Override protected void onStop(){foreground=false;resetConnection();super.onStop();}
    @Override protected void onDestroy(){cancelled=true;if(transfer!=null)transfer.cancel();network.shutdownNow();polls.shutdownNow();probes.shutdownNow();transfers.shutdownNow();super.onDestroy();}
    private void resetConnection(){connectionEpoch++;ui.removeCallbacks(pulse);nearby.stop();closeSocket();if(stateRequest!=null){stateRequest.cancel();stateRequest=null;}probing.clear();candidates.clear();}
    private void restartConnection(){
        resetConnection();retryTicks=0;setOnline(false);
        if(client!=null)try{client=new RoomClient(client.profile);}catch(Exception e){failure(e);}
        if(foreground){discover();ui.post(pulse);}
    }
    @Override protected void onNewIntent(Intent i){super.onNewIntent(i);setIntent(i);receive(i);consumePending();}
    private int dp(float n){return (int)(getResources().getDisplayMetrics().density*n+.5f);}
    private TextView label(String text,int size,int color){TextView t=new TextView(this);t.setText(text);t.setTextSize(size);t.setTextColor(color);t.setPadding(0,dp(6),0,dp(6));return t;}
    private GradientDrawable surface(int color){GradientDrawable d=new GradientDrawable();d.setColor(color);d.setCornerRadius(dp(22));d.setStroke(dp(1),0x334fe0cd);return d;}
    private LinearLayout column(){LinearLayout v=new LinearLayout(this);v.setOrientation(LinearLayout.VERTICAL);return v;}
    private Button button(String text,Runnable action){Button b=new Button(this);b.setText(text);b.setAllCaps(false);b.setTextColor(INK);b.setBackgroundTintList(android.content.res.ColorStateList.valueOf(0xff28434c));b.setOnClickListener(v->action.run());return b;}
    private void buildUi(){
        root=column();root.setPadding(dp(22),dp(18),dp(22),dp(12));root.setBackgroundColor(0xff101d27);
        ViewCompat.setOnApplyWindowInsetsListener(root,(v,insets)->{var bars=insets.getInsets(WindowInsetsCompat.Type.systemBars()|WindowInsetsCompat.Type.ime());v.setPadding(dp(22)+bars.left,dp(14)+bars.top,dp(22)+bars.right,dp(12)+bars.bottom);return insets;});
        TextView brand=label("PocketDrop",30,INK);brand.setTypeface(null,Typeface.BOLD);root.addView(brand);
        status=label("尚未配對 · 與電腦連上同一個 Wi-Fi",13,ACCENT);root.addView(status);
        notice=label("貼進去，丟進去，拿出來。",14,MUTED);root.addView(notice);
        LinearLayout tabs=new LinearLayout(this);String[] names={"文字","檔案","裝置"};for(int i=0;i<3;i++){final int index=i;tabs.addView(button(names[i],()->showTab(index)),new LinearLayout.LayoutParams(0,dp(50),1));}root.addView(tabs);
        ScrollView scroll=new ScrollView(this);scroll.setFillViewport(true);content=column();content.setPadding(0,dp(12),0,dp(12));scroll.addView(content);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        textView=column();textView.setPadding(dp(18),dp(14),dp(18),dp(14));textView.setBackground(surface(0xff192f3a));textView.addView(label("Shared Text",21,INK));
        editor=new EditText(this);editor.setTextColor(INK);editor.setHintTextColor(MUTED);editor.setHint("貼上文字，按分享，電腦就會看到");editor.setTextSize(17);editor.setGravity(Gravity.TOP);editor.setMinLines(7);editor.setMaxLines(14);editor.setInputType(android.text.InputType.TYPE_CLASS_TEXT|android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE|android.text.InputType.TYPE_TEXT_FLAG_CAP_SENTENCES);editor.setFilters(new InputFilter[]{new InputFilter.LengthFilter(32768)});textView.addView(editor,new LinearLayout.LayoutParams(-1,-2));
        editor.addTextChangedListener(new TextWatcher(){public void beforeTextChanged(CharSequence s,int st,int c,int a){}public void onTextChanged(CharSequence s,int st,int before,int count){if(!updating)editing=true;}public void afterTextChanged(Editable s){}});
        textView.addView(button("分享文字到 Room",()->sendText(editor.getText().toString())));
        LinearLayout row=new LinearLayout(this);row.addView(button("複製",()->{((ClipboardManager)getSystemService(CLIPBOARD_SERVICE)).setPrimaryClip(ClipData.newPlainText("PocketDrop",editor.getText()));notice("已複製");}),new LinearLayout.LayoutParams(0,-2,1));row.addView(button("清空並同步",()->sendText("")),new LinearLayout.LayoutParams(0,-2,1));textView.addView(row);
        textView.addView(button("載入 Room 最新文字",()->{editing=false;refresh();}));
        filesView=column();devicesView=column();
        progress=label("",12,MUTED);root.addView(progress);cancel=button("取消傳輸",()->{cancelled=true;Call c=transfer;if(c!=null)c.cancel();});cancel.setVisibility(View.GONE);root.addView(cancel);
        root.addView(label("1.0.1 · 建立 Room 的電腦需保持開啟",11,MUTED));setContentView(root);
    }
    private void showTab(int index){tab=index;content.removeAllViews();if(index==0)content.addView(textView);else if(index==1){renderFiles();content.addView(filesView);}else{renderDevices();content.addView(devicesView);}}
    private void notice(String text){if(!isDestroyed())ui.post(()->notice.setText(text));}
    private void failure(Exception e){String m=e.getMessage();notice(m==null?"操作失敗，請確認 Wi-Fi 與電腦狀態":m);}
    private void setOnline(boolean value){online=value;status.setText(client==null?"尚未配對 · 與電腦連上同一個 Wi-Fi":value?"● 已連線 · PocketDrop Room":"○ 目前無法取得 · 電腦離線或網路中斷");if(tab==1)renderFiles();}
    private void discover(){
        RoomClient c=client;if(c==null||!foreground)return;long epoch=connectionEpoch;
        nearby.start(c.profile.optString("device_id"),endpoint->{
            if(!foreground||connectionEpoch!=epoch||client!=c)return;
            try{candidates.observe(endpoint,SystemClock.elapsedRealtime());}catch(IllegalArgumentException ignored){return;}
            if(!online||!endpoint.equals(c.endpoint))retryCandidates();
        });
    }
    private void retryCandidates(){
        RoomClient c=client;if(c==null||!foreground||probes.isShutdown())return;long epoch=connectionEpoch;
        for(String endpoint:candidates.due(SystemClock.elapsedRealtime(),Math.max(0,2-probing.size()),probing)){
            if(online&&endpoint.equals(c.endpoint))continue;
            probing.add(endpoint);
            probes.execute(()->{try{
                RoomClient candidate=c.at(endpoint);candidate.state();
                ui.post(()->{
                    if(!foreground||connectionEpoch!=epoch||client!=c)return;
                    try{vault.save(candidate.profile);}catch(Exception e){failure(e);return;}
                    resetConnection();client=candidate;setOnline(true);notice("✓ 已找到電腦，沿用原有配對");
                    discover();ui.post(pulse);
                });
            }catch(Exception ignored){/* Failed candidates remain eligible for the next foreground retry. */}
            finally{ui.post(()->{if(connectionEpoch==epoch)probing.remove(endpoint);});}});
        }
    }
    private void connectSocket(){RoomClient c=client;if(c==null || socket!=null || !foreground)return;
        socket=c.http.newWebSocket(c.request("/v1/events").build(),new WebSocketListener(){
            public void onMessage(WebSocket ws,String text){ui.post(()->{if(client==c&&socket==ws)refresh();});}
            public void onFailure(WebSocket ws,Throwable t,Response r){ui.post(()->{if(socket==ws){socket=null;refresh();}});}
            public void onClosing(WebSocket ws,int code,String reason){ws.close(code,null);ui.post(()->{if(socket==ws)socket=null;});}
            public void onClosed(WebSocket ws,int code,String reason){ui.post(()->{if(socket==ws)socket=null;});}
        });
    }
    private void closeSocket(){if(socket!=null){socket.cancel();socket=null;}}
    private void refresh(){
        if(Looper.myLooper()!=Looper.getMainLooper()){ui.post(this::refresh);return;}
        RoomClient c=client;if(!foreground||isDestroyed()||polls.isShutdown()||c==null||stateRequest!=null)return;
        long epoch=connectionEpoch;Call call=c.stateCall();stateRequest=call;
        polls.execute(()->{try{JSONObject state=RoomClient.read(call);ui.post(()->{
            if(connectionEpoch!=epoch||client!=c||!foreground)return;
            boolean wasOnline=online;setOnline(true);if(!wasOnline)notice("✓ 已沿用原有配對重新連線");
            files=state.optJSONArray("files");if(files==null)files=new JSONArray();long next=state.optLong("revision");
            if(!editing){updating=true;editor.setText(state.optString("text"));updating=false;}
            else if(next!=revision&&revision>=0)notice("Room 有新文字；你的草稿已保留，可按「載入 Room 最新文字」");
            revision=next;if(tab==1)renderFiles();
        });}catch(Exception e){ui.post(()->{if(connectionEpoch==epoch&&client==c&&foreground){boolean wasOnline=online;setOnline(false);notice(RoomClient.connectionError(e));if(wasOnline){closeSocket();discover();}retryCandidates();}});}
        finally{ui.post(()->{if(stateRequest==call)stateRequest=null;});}});
    }
    private void sendText(String value){RoomClient c=client;if(c==null){notice("先到「裝置」掃描電腦的 QR Code");showTab(2);return;}
        network.execute(()->{try{c.text(value);ui.post(()->{if(client!=c)return;editing=false;updating=true;editor.setText(value);updating=false;notice("✓ 已分享文字");refresh();});}catch(Exception e){failure(e);}});
    }
    private void renderDevices(){devicesView.removeAllViews();devicesView.addView(label("自己的裝置，在同一個 Room",21,INK));devicesView.addView(label(client==null?"1. 電腦開啟 PocketDrop\n2. 按「邀請裝置」\n3. 用下方按鈕掃描 QR Code":"已保存電腦身分與配對。\n回到相同 Wi-Fi，開啟 App 便會重新尋找電腦。",15,MUTED));devicesView.addView(button(client==null?"掃描電腦 QR Code":"重新掃描 QR Code",()->new IntentIntegrator(this).setDesiredBarcodeFormats(IntentIntegrator.QR_CODE).setPrompt("掃描 PocketDrop 電腦畫面的 QR Code").setBeepEnabled(false).setOrientationLocked(false).initiateScan()));
        devicesView.addView(button("輸入驗證碼加入",this::pairByCode));
        if(client!=null){devicesView.addView(button("重新尋找電腦",()->{restartConnection();}));devicesView.addView(button("忘記此 Room",()->new AlertDialog.Builder(this).setTitle("忘記此 Room？").setMessage("手機將移除配對。若要撤銷存取權，請在電腦移除此裝置。").setNegativeButton("取消",null).setPositiveButton("忘記",(d,w)->{cancelled=true;if(transfer!=null)transfer.cancel();pairingAttempt++;resetConnection();vault.forget();client=null;files=new JSONArray();revision=-1;editing=false;editor.setText("");nearby.stop();closeSocket();setOnline(false);renderDevices();}).show()));}
        devicesView.addView(label("此 Room 由建立它的電腦保存。\n不使用雲端，不需帳號。\n開啟 App 後會自動連線。",13,MUTED));
    }
    private void pairByCode(){
        LinearLayout panel=column();panel.setPadding(dp(18),dp(10),dp(18),dp(10));panel.addView(label("請在電腦按「邀請裝置」，再選擇相同編號。",14,MUTED));
        Spinner rooms=new Spinner(this);ArrayList<String> names=new ArrayList<>(),addresses=new ArrayList<>();ArrayAdapter<String> adapter=new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,names);rooms.setAdapter(adapter);panel.addView(rooms);
        EditText code=new EditText(this);code.setHint("8 位驗證碼");code.setInputType(android.text.InputType.TYPE_CLASS_NUMBER);code.setFilters(new InputFilter[]{new InputFilter.LengthFilter(8)});panel.addView(code);
        TextView message=label("正在尋找附近的電腦…",13,MUTED);panel.addView(message);
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle("驗證碼配對").setView(panel).setNegativeButton("取消",null).setPositiveButton("加入",null).create();
        dialog.setOnDismissListener(d->{nearby.stop();if(client!=null&&foreground)discover();});dialog.setOnShowListener(d->{nearby.start(null,result->{String[] parts=result.split("\\|",2);if(parts.length!=2||addresses.contains(parts[1])||names.size()>=32)return;names.add("PocketDrop · "+parts[0].substring(0,Math.min(8,parts[0].length())));addresses.add(parts[1]);adapter.notifyDataSetChanged();message.setText("選擇電腦，輸入它顯示的驗證碼。");});
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(v->{if(addresses.isEmpty()){message.setText("尚未找到電腦，請確認同一個 Wi-Fi。");return;}String value=code.getText().toString();if(!value.matches("[0-9]{8}")){message.setText("請輸入 8 位驗證碼");return;}String endpoint=addresses.get(rooms.getSelectedItemPosition());long attempt=++pairingAttempt;dialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(false);message.setText("正在驗證電腦身分…");network.execute(()->{try{RoomClient paired=CodePairing.pair(endpoint,value,vault.deviceId(),Build.MODEL.substring(0,Math.min(60,Build.MODEL.length())),vault.publicKey());ui.post(()->{if(isDestroyed()||attempt!=pairingAttempt)return;try{vault.save(paired.profile);}catch(Exception e){failure(e);return;}client=paired;revision=-1;editing=false;dialog.dismiss();restartConnection();notice("✓ 已配對，之後會自動連線");renderDevices();consumePending();});}catch(Exception e){ui.post(()->{if(dialog.isShowing()){message.setText("配對失敗：請確認驗證碼，或在電腦產生新邀請。");dialog.getButton(AlertDialog.BUTTON_POSITIVE).setEnabled(true);}});}});});});dialog.show();
    }
    private void pair(String qr){long attempt=++pairingAttempt;network.execute(()->{try{
        if(qr.length()>4096)throw new IllegalArgumentException("QR Code 過大");JSONObject p=new JSONObject(qr);
        if(!"PocketDrop".equals(p.optString("app")) || p.optInt("protocol_version")!=1 || !"phone-trial".equals(p.optString("mode")))throw new IllegalArgumentException("不是 PocketDrop QR Code");
        UUID.fromString(p.getString("device_id"));UUID.fromString(p.getString("room_id"));if(!p.getString("token").matches("[A-Za-z0-9_-]{43}"))throw new IllegalArgumentException("邀請格式錯誤");
        RoomClient c=new RoomClient(p);JSONObject body=new JSONObject().put("token",p.getString("token")).put("device_id",vault.deviceId()).put("name",android.os.Build.MODEL.substring(0,Math.min(60,android.os.Build.MODEL.length()))).put("public_key",vault.publicKey());
        JSONObject result=c.json(new Request.Builder().url(c.endpoint+"/v1/pair").post(RequestBody.create(body.toString(),RoomClient.JSON)).build());
        if(!p.getString("room_id").equals(result.getString("room_id")) || !p.getString("device_id").equals(result.getString("device_id")) || !result.getString("credential").matches("[A-Za-z0-9_-]{43}"))throw new IllegalArgumentException("電腦回應不符配對資料");
        p.remove("token");p.put("credential",result.getString("credential"));RoomClient paired=new RoomClient(p);ui.post(()->{
            if(isDestroyed()||attempt!=pairingAttempt)return;
            try{vault.save(paired.profile);}catch(Exception e){failure(e);return;}
            client=paired;revision=-1;editing=false;restartConnection();notice("✓ 已配對，之後不用重掃");renderDevices();consumePending();
        });
    }catch(Exception e){failure(e);}});}
    @Override protected void onActivityResult(int request,int result,Intent data){super.onActivityResult(request,result,data);IntentResult scan=IntentIntegrator.parseActivityResult(request,result,data);if(scan!=null){if(scan.getContents()!=null)pair(scan.getContents());return;}if(request==PICK && result==RESULT_OK && data!=null){ArrayList<Uri> uris=new ArrayList<>();if(data.getClipData()!=null){for(int i=0;i<Math.min(100,data.getClipData().getItemCount());i++)uris.add(data.getClipData().getItemAt(i).getUri());}else if(data.getData()!=null)uris.add(data.getData());uploadAll(uris);}}
    @SuppressWarnings("deprecation") private void receive(Intent intent){
        if(intent==null)return;String action=intent.getAction();if(!Intent.ACTION_SEND.equals(action)&&!Intent.ACTION_SEND_MULTIPLE.equals(action))return;
        CharSequence text=intent.getCharSequenceExtra(Intent.EXTRA_TEXT);if(text!=null)pendingText=text.toString();
        if(Intent.ACTION_SEND.equals(action)){Uri uri=intent.getParcelableExtra(Intent.EXTRA_STREAM);if(uri!=null)pendingUris.add(uri);}else{ArrayList<Uri> uris=intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM);if(uris!=null)pendingUris.addAll(uris.subList(0,Math.min(100,uris.size())));}
        if(client==null)notice("已收到分享內容，先掃描電腦 QR Code 即可加入 Room");
    }
    private void consumePending(){if(client==null)return;if(pendingText!=null){String text=pendingText;pendingText=null;updating=true;editor.setText(text);updating=false;sendText(text);showTab(0);}if(!pendingUris.isEmpty()&&!transferBusy){ArrayList<Uri> items=new ArrayList<>(pendingUris);pendingUris.clear();uploadAll(items);showTab(1);}}
    private void renderFiles(){filesView.removeAllViews();filesView.addView(label("Shared Files",21,INK));filesView.addView(button("加入手機檔案",()->{if(client==null){showTab(2);notice("請先配對電腦");return;}Intent pick=new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE).putExtra(Intent.EXTRA_ALLOW_MULTIPLE,true);startActivityForResult(pick,PICK);}));
        if(files.length()==0)filesView.addView(label("把檔案拖進電腦 Widget，或從手機分享進來。只有按下載才會存到手機。",15,MUTED));
        for(int i=0;i<files.length();i++){JSONObject f=files.optJSONObject(i);if(f==null)continue;LinearLayout card=column();card.setPadding(dp(14),dp(10),dp(14),dp(10));card.setBackground(surface(0xff192f3a));LinearLayout.LayoutParams params=new LinearLayout.LayoutParams(-1,-2);params.bottomMargin=dp(10);filesView.addView(card,params);card.addView(label(f.optString("name"),17,INK));boolean available=online&&f.optBoolean("available");card.addView(label(size(f.optLong("size"))+" · "+f.optString("origin")+(!available?"\n目前無法取得 · 來源離線或檔案已變更":""),12,MUTED));Button download=button("下載",()->download(f));download.setEnabled(available&&!transferBusy);card.addView(download);}
    }
    private static String size(long n){if(n<1024)return n+" B";if(n<1024*1024)return String.format(Locale.ROOT,"%.1f KB",n/1024.0);if(n<1024L*1024*1024)return String.format(Locale.ROOT,"%.1f MB",n/(1024.0*1024));return String.format(Locale.ROOT,"%.1f GB",n/(1024.0*1024*1024));}
    private boolean beginTransfer(){if(client==null){notice("請先配對電腦");return false;}if(transferBusy){notice("請等待目前傳輸完成，或先取消");return false;}transferBusy=true;cancelled=false;cancel.setVisibility(View.VISIBLE);progress.setText("準備傳輸…");if(tab==1)renderFiles();return true;}
    private void endTransfer(){transfer=null;ui.post(()->{transferBusy=false;cancel.setVisibility(View.GONE);if(tab==1)renderFiles();});}
    private void updateProgress(String name,long done,long total,long started){long elapsed=Math.max(1,SystemClock.elapsedRealtime()-started);long speed=done*1000/elapsed;long remain=speed==0?0:Math.max(0,total-done)/speed;ui.post(()->progress.setText(name+"\n"+size(done)+" / "+size(total)+" · "+size(speed)+"/s · 約 "+remain+" 秒"));}
    private void uploadAll(ArrayList<Uri> uris){if(uris.isEmpty()||!beginTransfer())return;RoomClient c=client;transfers.execute(()->{try{int count=0;for(Uri uri:uris){if(cancelled)throw new IOException("已取消傳輸");upload(c,uri);count++;}notice("✓ "+count+" 個檔案已加入共享區；電腦下載資料夾內的 PocketDrop 可取得");refresh();}catch(Exception e){if(cancelled)notice("已取消傳輸");else failure(e);}finally{endTransfer();}});}
    private void upload(RoomClient c,Uri uri)throws Exception{
        if(!"content".equals(uri.getScheme()))throw new IOException("請透過系統檔案選擇器加入檔案");String name=null;long length=-1;
        try(Cursor cursor=getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME,OpenableColumns.SIZE},null,null,null)){if(cursor!=null&&cursor.moveToFirst()){int ni=cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME),si=cursor.getColumnIndex(OpenableColumns.SIZE);if(ni>=0)name=cursor.getString(ni);if(si>=0&&!cursor.isNull(si))length=cursor.getLong(si);}}
        if(!LanRules.safeName(name))throw new IOException("此檔名無法安全儲存，請先重新命名");if(length<0)throw new IOException("無法得知檔案大小，請先將檔案存到手機再分享");if(length>16L*1024*1024*1024)throw new IOException("單檔最多 16 GiB");final long total=length;final String filename=name;
        RequestBody body=new RequestBody(){public MediaType contentType(){return MediaType.get("application/octet-stream");}public long contentLength(){return total;}public void writeTo(BufferedSink sink)throws IOException{long done=0,started=SystemClock.elapsedRealtime(),last=0;try(InputStream input=getContentResolver().openInputStream(uri)){if(input==null)throw new IOException("無法讀取檔案");byte[] buffer=new byte[65536];int n;while((n=input.read(buffer))!=-1){if(cancelled)throw new IOException("已取消");sink.write(buffer,0,n);done+=n;long now=SystemClock.elapsedRealtime();if(now-last>250){updateProgress(filename,done,total,started);last=now;}}if(done!=total)throw new IOException("來源檔案大小已變更");}}};
        HttpUrl url=Objects.requireNonNull(HttpUrl.parse(c.endpoint+"/v1/files")).newBuilder().addQueryParameter("name",filename).build();transfer=c.http.newCall(c.request("/v1/files").url(url).header("x-file-size",Long.toString(total)).put(body).build());try(Response response=transfer.execute()){RoomClient.check(response);}
    }
    private void download(JSONObject item){if(!beginTransfer())return;RoomClient c=client;transfers.execute(()->{Uri target=null;File legacy=null;boolean complete=false;try{
        String name=item.getString("name");if(!LanRules.safeName(name))throw new IOException("不安全的檔名");String id=item.getString("file_id");UUID.fromString(id);long expected=item.getLong("size");if(expected<0||expected>16L*1024*1024*1024)throw new IOException("檔案大小超過限制");
        transfer=c.http.newCall(c.request("/v1/files/"+id).build());try(Response response=transfer.execute()){RoomClient.check(response);if(response.body()==null || response.body().contentLength()!=expected)throw new IOException("檔案大小不符");
            if(Build.VERSION.SDK_INT>=29){ContentValues v=new ContentValues();v.put(MediaStore.Downloads.DISPLAY_NAME,name);v.put(MediaStore.Downloads.MIME_TYPE,"application/octet-stream");v.put(MediaStore.Downloads.RELATIVE_PATH,Environment.DIRECTORY_DOWNLOADS+"/PocketDrop");v.put(MediaStore.Downloads.IS_PENDING,1);target=getContentResolver().insert(MediaStore.Downloads.EXTERNAL_CONTENT_URI,v);if(target==null)throw new IOException("無法建立下載檔案");}
            else{File dir=new File(getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS),"PocketDrop/"+UUID.randomUUID());if(!dir.mkdirs())throw new IOException("無法建立資料夾");legacy=new File(dir,name);target=FileProvider.getUriForFile(this,getPackageName()+".files",legacy);}
            MessageDigest digest=MessageDigest.getInstance("SHA-256");long done=0,started=SystemClock.elapsedRealtime(),last=0;
            try(InputStream in=response.body().byteStream();OutputStream out=Build.VERSION.SDK_INT>=29?getContentResolver().openOutputStream(target):new FileOutputStream(legacy)){if(out==null)throw new IOException("無法儲存檔案");byte[] buffer=new byte[65536];int n;while((n=in.read(buffer))!=-1){if(cancelled)throw new IOException("已取消");done+=n;if(done>expected)throw new IOException("內容超出宣告大小");out.write(buffer,0,n);digest.update(buffer,0,n);long now=SystemClock.elapsedRealtime();if(now-last>250){updateProgress(name,done,expected,started);last=now;}}}
            if(done!=expected)throw new IOException("下載不完整");String checksum=response.header("x-content-sha256");if(checksum!=null&&!checksum.equals(RoomClient.hex(digest.digest())))throw new IOException("完整性驗證失敗，已移除下載檔案");if(cancelled)throw new IOException("已取消");
            if(Build.VERSION.SDK_INT>=29){ContentValues values=new ContentValues();values.put(MediaStore.Downloads.IS_PENDING,0);getContentResolver().update(target,values,null,null);}complete=true;Uri saved=target;
            ui.post(()->{progress.setText("✓ 下載完成"+(checksum==null?" · 來源雜湊計算中，尚未比對":" · SHA-256 已驗證"));new AlertDialog.Builder(this).setTitle("下載完成").setMessage(name+"\n"+(Build.VERSION.SDK_INT>=29?"已存入 Downloads/PocketDrop":"已存入 PocketDrop 的 App 下載資料夾")).setNegativeButton("完成",null).setPositiveButton("開啟",(d,w)->openFile(saved,name)).show();});
        }
    }catch(Exception e){if(cancelled)notice("已取消下載");else failure(e);}finally{if(!complete){if(legacy!=null){if(!legacy.delete())notice("下載中斷，請檢查儲存空間");}else if(target!=null)getContentResolver().delete(target,null,null);}endTransfer();}});}
    private void openFile(Uri uri,String name){String ext=name.contains(".")?name.substring(name.lastIndexOf('.')+1).toLowerCase(Locale.ROOT):"";if(Arrays.asList("apk","exe","msi","bat","cmd","ps1","com","scr").contains(ext)){notice("此類檔案僅供儲存，請自行到檔案管理器處理");return;}String mime=android.webkit.MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext);try{startActivity(new Intent(Intent.ACTION_VIEW).setDataAndType(uri,mime==null?"application/octet-stream":mime).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION));}catch(ActivityNotFoundException e){notice("手機沒有可開啟此格式的 App，檔案已保存");}}
}
