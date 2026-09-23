package local.pocketdrop.android;

import org.json.JSONObject;
import org.junit.Test;
import static org.junit.Assert.*;
import java.net.*;
import java.util.*;
import java.util.concurrent.*;
import okhttp3.Call;
import okhttp3.mockwebserver.*;
import okhttp3.tls.*;

/** Real TLS/HTTP using the production Android client; no QR endpoint exists. */
public class ReconnectTest {
    private InetAddress address()throws Exception {
        for(NetworkInterface n:Collections.list(NetworkInterface.getNetworkInterfaces()))
            for(InetAddress a:Collections.list(n.getInetAddresses()))
                if(n.isUp()&&LanRules.privateHost(a.getHostAddress()))return a;
        throw new IllegalStateException("LAN integration tests require a private IPv4 interface");
    }
    private final String credential="c".repeat(43);
    private final HeldCertificate certificate=new HeldCertificate.Builder().commonName("PocketDrop test").build();
    private MockWebServer server(HeldCertificate cert)throws Exception {
        MockWebServer server=new MockWebServer();
        server.useHttps(new HandshakeCertificates.Builder().heldCertificate(cert).build().sslSocketFactory(),false);
        server.setDispatcher(new Dispatcher(){public MockResponse dispatch(RecordedRequest request){
            if(!("Bearer "+credential).equals(request.getHeader("Authorization")))return new MockResponse().setResponseCode(401);
            if(!"/v1/state".equals(request.getPath()))return new MockResponse().setResponseCode(404);
            return new MockResponse().setBody("{\"revision\":7,\"text\":\"restored\",\"files\":[]}");
        }});
        server.start(address(),0);return server;
    }
    private String endpoint(MockWebServer s)throws Exception {return "https://"+address().getHostAddress()+":"+s.getPort();}
    private JSONObject profile(MockWebServer server)throws Exception {
        return new JSONObject().put("endpoint",endpoint(server)).put("credential",credential)
            .put("device_id",UUID.randomUUID().toString()).put("room_id",UUID.randomUUID().toString())
            .put("certificate_sha256",RoomClient.hex(java.security.MessageDigest.getInstance("SHA-256").digest(certificate.certificate().getEncoded())));
    }
    @Test public void coldStartRestoresCredentialWithoutPairing()throws Exception {
        try(MockWebServer server=server(certificate)){
            JSONObject saved=profile(server);
            RoomClient first=new RoomClient(saved);assertEquals(7,first.state().getInt("revision"));
            // New JSON and HTTP/TLS client, as after Activity/process recreation.
            RoomClient reopened=new RoomClient(new JSONObject(saved.toString()));
            assertEquals("restored",reopened.state().getString("text"));
            assertEquals("/v1/state",server.takeRequest().getPath());
            assertEquals("Bearer "+credential,server.takeRequest().getHeader("Authorization"));
            assertEquals(2,server.getRequestCount());assertFalse(reopened.profile.has("token"));
        }
    }
    @Test public void changedPortKeepsOriginalTrustAndCredentials()throws Exception {
        JSONObject saved;
        try(MockWebServer first=server(certificate)){saved=profile(first);assertEquals(7,new RoomClient(saved).state().getInt("revision"));}
        try(MockWebServer second=server(certificate)){
            RoomClient stale=new RoomClient(new JSONObject(saved.toString()));
            RoomClient rediscovered=stale.at(endpoint(second));
            assertEquals(7,rediscovered.state().getInt("revision"));
            assertEquals(credential,rediscovered.profile.getString("credential"));
            assertEquals(saved.getString("endpoint"),stale.endpoint);
            assertEquals(saved.getString("certificate_sha256"),rediscovered.profile.getString("certificate_sha256"));
        }
    }
    @Test public void discoveryCannotReplacePinnedIdentity()throws Exception {
        try(MockWebServer trusted=server(certificate);MockWebServer imposter=server(new HeldCertificate.Builder().commonName("wrong device").build())){
            RoomClient original=new RoomClient(profile(trusted));
            try{original.at(endpoint(imposter)).state();fail("Different TLS identity accepted");}catch(javax.net.ssl.SSLException expected){}
            assertEquals(7,original.state().getInt("revision"));
        }
    }
    @Test public void stoppedRequestCannotBlockForegroundReconnect()throws Exception {
        try(MockWebServer server=server(certificate)){
            server.setDispatcher(new QueueDispatcher());
            server.enqueue(new MockResponse().setSocketPolicy(SocketPolicy.NO_RESPONSE));
            server.enqueue(new MockResponse().setBody("{\"revision\":8}"));
            JSONObject saved=profile(server);Call old=new RoomClient(saved).stateCall();
            ExecutorService executor=Executors.newSingleThreadExecutor();
            try{
                Future<?> pending=executor.submit(()->{try{RoomClient.read(old);}catch(Exception expected){}});
                assertNotNull(server.takeRequest(3,TimeUnit.SECONDS));old.cancel();pending.get(3,TimeUnit.SECONDS);
                assertEquals(8,new RoomClient(new JSONObject(saved.toString())).state().getInt("revision"));
            }finally{old.cancel();executor.shutdownNow();}
        }
    }
}
