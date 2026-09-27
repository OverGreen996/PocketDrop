package local.pocketdrop.android;

import org.json.JSONObject;
import java.net.URI;
import java.io.*;
import java.security.MessageDigest;

final class UpdateRelease {
    static final long MAX_APK = 128L * 1024 * 1024;
    final int code, minSdk;
    final long size;
    final String version, url, hash, notes;
    UpdateRelease(JSONObject j) throws Exception {
        code=j.getInt("versionCode"); minSdk=j.getInt("minSdk"); size=j.getLong("size");
        version=j.getString("versionName"); url=j.getString("url"); hash=j.getString("sha256"); notes=j.optString("notes","");
        URI u=new URI(url);
        if(code<1 || minSdk<26 || size<1 || size>MAX_APK || !version.matches("[0-9]+\\.[0-9]+\\.[0-9]+") || !hash.matches("[0-9a-f]{64}") || notes.length()>12000
          || !"https".equals(u.getScheme()) || !"github.com".equals(u.getHost()) || u.getPort()!=-1 || u.getUserInfo()!=null || u.getQuery()!=null || u.getFragment()!=null
          || !u.getPath().startsWith("/OverGreen996/PocketDrop/releases/download/") || !u.getPath().endsWith(".apk")) throw new IOException("無效更新資訊");
    }
    boolean newer(long current, int sdk) { return code>current && sdk>=minSdk; }
    interface Progress { void update(long done,long total) throws IOException; }
    void copyVerified(InputStream in, OutputStream out, Progress progress) throws Exception {
        MessageDigest digest=MessageDigest.getInstance("SHA-256"); byte[] buffer=new byte[65536]; long done=0; int n;
        while((n=in.read(buffer))!=-1) { done+=n; if(done>size)throw new IOException("更新大小不符"); digest.update(buffer,0,n);out.write(buffer,0,n);progress.update(done,size); }
        StringBuilder actual=new StringBuilder();for(byte b:digest.digest())actual.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
        if(done!=size || !hash.equals(actual.toString()))throw new IOException("更新完整性驗證失敗");
    }
}
