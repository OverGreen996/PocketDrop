package local.pocketdrop.android;

import org.bouncycastle.crypto.agreement.srp.SRP6Client;
import org.bouncycastle.crypto.agreement.srp.SRP6StandardGroups;
import org.bouncycastle.crypto.digests.SHA256Digest;
import org.json.JSONObject;
import okhttp3.*;
import javax.net.ssl.*;
import java.security.*;
import java.security.cert.X509Certificate;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.concurrent.TimeUnit;

/** Standard SRP-6a proofs, bound to the observed TLS leaf and requesting device. */
final class CodePairing {
    private static byte[] bytes(BigInteger n,int length){byte[] raw=n.toByteArray(),out=new byte[length];int count=Math.min(raw.length,length);System.arraycopy(raw,raw.length-count,out,length-count,count);return out;}
    private static BigInteger integer(String s){byte[] b=Base64.getDecoder().decode(s);if(b.length==0||b.length>256)throw new IllegalArgumentException("配對回應無效");return new BigInteger(1,b);}
    static RoomClient pair(String address,String code,String deviceId,String name,String publicKey)throws Exception{
        String endpoint=LanRules.endpoint(address);if(!code.matches("[0-9]{8}"))throw new IllegalArgumentException("請輸入 8 位驗證碼");
        X509TrustManager bootstrapTrust=new X509TrustManager(){public X509Certificate[] getAcceptedIssuers(){return new X509Certificate[0];}public void checkClientTrusted(X509Certificate[] c,String a)throws java.security.cert.CertificateException{throw new java.security.cert.CertificateException("server only");}public void checkServerTrusted(X509Certificate[] c,String a)throws java.security.cert.CertificateException{if(c.length==0)throw new java.security.cert.CertificateException("missing certificate");c[0].checkValidity();}};
        SSLContext ssl=SSLContext.getInstance("TLS");ssl.init(null,new TrustManager[]{bootstrapTrust},null);
        // No content or credentials are sent by this bootstrap-only client.
        java.util.concurrent.atomic.AtomicReference<byte[]> observedLeaf=new java.util.concurrent.atomic.AtomicReference<>();
        OkHttpClient bootstrap=new OkHttpClient.Builder().sslSocketFactory(ssl.getSocketFactory(),bootstrapTrust).hostnameVerifier((host,session)->{if(!LanRules.privateHost(host))return false;try{observedLeaf.set(session.getPeerCertificates()[0].getEncoded());return true;}catch(Exception e){return false;}}).proxy(java.net.Proxy.NO_PROXY).followRedirects(false).followSslRedirects(false).callTimeout(8,TimeUnit.SECONDS).build();
        JSONObject p;String pin;
        try(Response response=bootstrap.newCall(new Request.Builder().url(endpoint+"/v1/hello").build()).execute()){
            RoomClient.check(response);if(response.handshake()==null||response.body()==null)throw new IllegalArgumentException("無法驗證電腦");
            if(observedLeaf.get()==null)throw new IllegalArgumentException("無法驗證電腦憑證");pin=RoomClient.hex(MessageDigest.getInstance("SHA-256").digest(observedLeaf.get()));
            p=new JSONObject(response.body().string());
        }finally{bootstrap.connectionPool().evictAll();}
        if(!"PocketDrop".equals(p.optString("app"))||p.optInt("protocol_version")!=1)throw new IllegalArgumentException("電腦版本不支援驗證碼");
        java.util.UUID.fromString(p.getString("device_id"));java.util.UUID.fromString(p.getString("room_id"));
        p.put("endpoint",endpoint).put("certificate_sha256",pin);RoomClient pinned=new RoomClient(p);
        // SRP generates A before the server salt is available. The server returns
        // salt only after receiving A; retain the same private exponent and fill x
        // by using a small subclass designed for this normal two-message exchange.
        Exchange exchange=new Exchange();exchange.init(SRP6StandardGroups.rfc5054_2048,new SHA256Digest(),new SecureRandom());
        BigInteger a=exchange.begin();JSONObject start=new JSONObject().put("device_id",deviceId).put("name",name).put("public_key",publicKey).put("a",Base64.getEncoder().encodeToString(bytes(a,256)));
        JSONObject ch=pinned.json(new Request.Builder().url(endpoint+"/v1/code/start").post(RequestBody.create(start.toString(),RoomClient.JSON)).build());
        byte[] salt=Base64.getDecoder().decode(ch.getString("salt"));if(salt.length!=32)throw new IllegalArgumentException("配對回應無效");
        String identity="PocketDrop-1\n"+p.getString("device_id")+"\n"+pin+"\n"+deviceId+"\n"+name+"\n"+publicKey;
        exchange.password(salt,identity.getBytes(StandardCharsets.UTF_8),code.getBytes(StandardCharsets.US_ASCII));exchange.calculateSecret(integer(ch.getString("b")));
        String proof=Base64.getEncoder().encodeToString(bytes(exchange.calculateClientEvidenceMessage(),32));
        JSONObject result=pinned.json(new Request.Builder().url(endpoint+"/v1/code/finish").post(RequestBody.create(new JSONObject().put("id",ch.getString("id")).put("proof",proof).toString(),RoomClient.JSON)).build());
        if(!exchange.verifyServerEvidenceMessage(integer(result.getString("proof"))))throw new IllegalArgumentException("驗證碼或電腦身分不符");
        JSONObject paired=result.getJSONObject("paired");if(!p.getString("device_id").equals(paired.getString("device_id"))||!p.getString("room_id").equals(paired.getString("room_id"))||!paired.getString("credential").matches("[A-Za-z0-9_-]{43}"))throw new IllegalArgumentException("Room 身分不符");
        p.put("credential",paired.getString("credential"));return new RoomClient(p);
    }
    private static final class Exchange extends SRP6Client {
        BigInteger begin(){a=selectPrivateValue();A=g.modPow(a,N);return A;}
        void password(byte[] salt,byte[] identity,byte[] password){x=org.bouncycastle.crypto.agreement.srp.SRP6Util.calculateX(digest,N,salt,identity,password);}
    }
}
