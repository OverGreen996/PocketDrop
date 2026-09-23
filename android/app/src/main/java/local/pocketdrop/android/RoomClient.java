package local.pocketdrop.android;

import org.json.JSONObject;
import java.security.MessageDigest;
import java.security.cert.CertificateException;
import java.security.cert.X509Certificate;
import java.util.concurrent.TimeUnit;
import javax.net.ssl.SSLContext;
import javax.net.ssl.X509TrustManager;
import okhttp3.*;

final class RoomClient {
    static final MediaType JSON=MediaType.get("application/json; charset=utf-8");
    final OkHttpClient http;
    final OkHttpClient control;
    final JSONObject profile;
    final String endpoint;
    RoomClient(JSONObject p) throws Exception {
        profile=new JSONObject(p.toString());endpoint=LanRules.endpoint(p.getString("endpoint"));
        String pin=p.getString("certificate_sha256");if(!pin.matches("[0-9a-f]{64}"))throw new IllegalArgumentException("憑證格式錯誤");
        X509TrustManager trust=new X509TrustManager(){
            public void checkClientTrusted(X509Certificate[] c,String a)throws CertificateException{throw new CertificateException("server only");}
            public X509Certificate[] getAcceptedIssuers(){return new X509Certificate[0];}
            public void checkServerTrusted(X509Certificate[] chain,String auth)throws CertificateException{
                if(chain==null || chain.length==0)throw new CertificateException("no certificate");chain[0].checkValidity();
                try{if(!MessageDigest.isEqual(pin.getBytes(java.nio.charset.StandardCharsets.US_ASCII),hex(MessageDigest.getInstance("SHA-256").digest(chain[0].getEncoded())).getBytes(java.nio.charset.StandardCharsets.US_ASCII)))throw new CertificateException("配對電腦的身分不符");}catch(CertificateException e){throw e;}catch(Exception e){throw new CertificateException(e);}
            }
        };
        SSLContext ssl=SSLContext.getInstance("TLS");ssl.init(null,new javax.net.ssl.TrustManager[]{trust},null);
        http=new OkHttpClient.Builder().proxy(java.net.Proxy.NO_PROXY).sslSocketFactory(ssl.getSocketFactory(),trust).hostnameVerifier((host,session)->{
            if(!LanRules.privateHost(host))return false;
            try{trust.checkServerTrusted(new X509Certificate[]{(X509Certificate)session.getPeerCertificates()[0]},"RSA");return true;}catch(Exception e){return false;}
        }).followRedirects(false).followSslRedirects(false).connectTimeout(5,TimeUnit.SECONDS).readTimeout(30,TimeUnit.SECONDS).writeTimeout(30,TimeUnit.SECONDS).build();
        control=http.newBuilder().callTimeout(8,TimeUnit.SECONDS).readTimeout(8,TimeUnit.SECONDS).build();
    }
    static String hex(byte[] bytes){StringBuilder s=new StringBuilder();for(byte b:bytes)s.append(String.format(java.util.Locale.ROOT,"%02x",b & 255));return s.toString();}
    Request.Builder request(String path){return new Request.Builder().url(endpoint+path).header("Authorization","Bearer "+profile.optString("credential")).header("x-pocketdrop-room",profile.optString("room_id"));}
    JSONObject json(Request request)throws Exception{
        return read(control.newCall(request));
    }
    static JSONObject read(Call call)throws Exception {try(Response r=call.execute()){check(r);String body=r.body()==null?"":r.body().string();return body.isEmpty()?new JSONObject():new JSONObject(body);}}
    Call stateCall(){return control.newCall(request("/v1/state").build());}
    RoomClient at(String address)throws Exception {return new RoomClient(new JSONObject(profile.toString()).put("endpoint",LanRules.endpoint(address)));}
    static String connectionError(Exception e){
        if(e instanceof ApiException)return e.getMessage();
        if(e instanceof javax.net.ssl.SSLException)return "電腦身分驗證未通過；已保留配對，請確認電腦未重設 Room";
        return "正在重新尋找電腦；請保持電腦開啟並使用同一個 Wi-Fi（配對已保存）";
    }
    static final class ApiException extends java.io.IOException {final int status;ApiException(int code,String message){super(message);status=code;}}
    static void check(Response r)throws java.io.IOException{
        if(!r.isSuccessful())throw new ApiException(r.code(),switch(r.code()){case 401->"電腦已撤銷或更換此配對，請在電腦確認裝置紀錄";case 403->"無法存取此 Room";case 404,410->"目前無法取得，來源檔案已移動或刪除";case 413->"內容超過大小限制";case 429->"電腦正在接收其他檔案，請稍後再試";default->"操作失敗（"+r.code()+"）";});
    }
    JSONObject state()throws Exception{return json(request("/v1/state").build());}
    void text(String value)throws Exception{if(value.getBytes(java.nio.charset.StandardCharsets.UTF_8).length>32768)throw new IllegalArgumentException("文字最多 32 KB");json(request("/v1/text").put(RequestBody.create(new JSONObject().put("content",value).toString(),JSON)).build());}
}
