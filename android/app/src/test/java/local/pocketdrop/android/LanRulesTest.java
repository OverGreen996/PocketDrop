package local.pocketdrop.android;
import org.junit.Test;
import static org.junit.Assert.*;
public class LanRulesTest {
 @Test public void onlyPrivateEndpoints()throws Exception {assertEquals("https://192.168.1.2:9876",LanRules.endpoint("https://192.168.1.2:9876"));for(String value:new String[]{"http://192.168.1.2:9","https://example.com:9","https://127.0.0.1:9","https://8.8.8.8:9","https://192.168.1.2:9@8.8.8.8:9","https://192.168.1.2:9/a","https://192.168.1.2:9?x=y","https://010.1.1.1:9"}){try{LanRules.endpoint(value);fail(value);}catch(IllegalArgumentException|java.net.URISyntaxException expected){}}}
 @Test public void dangerousNames(){for(String n:new String[]{"../a","C:\\x","CON.txt","file.","a\nb","NUL","x/y",""})assertFalse(n,LanRules.safeName(n));assertTrue(LanRules.safeName("模型.stl"));assertTrue(LanRules.safeName("upload.part"));}
}
