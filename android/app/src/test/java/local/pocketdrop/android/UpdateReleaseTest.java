package local.pocketdrop.android;
import org.junit.Test;
import org.json.JSONObject;
import java.io.*;
import java.security.MessageDigest;
import static org.junit.Assert.*;

public class UpdateReleaseTest {
    private JSONObject manifest(byte[] bytes)throws Exception{
        StringBuilder h=new StringBuilder();for(byte b:MessageDigest.getInstance("SHA-256").digest(bytes))h.append(String.format("%02x",b&255));
        return new JSONObject().put("versionCode",6).put("versionName","1.0.2").put("minSdk",26).put("size",bytes.length).put("sha256",h.toString()).put("url","https://github.com/OverGreen996/PocketDrop/releases/download/android-v1.0.2/PocketDrop-1.0.2-Android.apk");
    }
    @Test public void acceptsNewerAndRejectsDowngradeOrUnsupportedSdk()throws Exception{
        UpdateRelease release=new UpdateRelease(manifest(new byte[]{1,2,3}));
        assertTrue(release.newer(5,26));assertFalse(release.newer(6,35));assertFalse(release.newer(7,35));assertFalse(release.newer(5,25));
    }
    @Test public void rejectsInsecureAndUnrelatedSources()throws Exception{
        for(String url:new String[]{"http://github.com/OverGreen996/PocketDrop/releases/download/v/a.apk","https://evil.example/a.apk","https://github.com/another/repo/a.apk","https://github.com@evil.example/a.apk"}){
            try{new UpdateRelease(manifest(new byte[]{1}).put("url",url));fail(url);}catch(IOException expected){}
        }
    }
    @Test public void rejectsInvalidMetadata()throws Exception{
        for(JSONObject j:new JSONObject[]{manifest(new byte[]{1}).put("size",UpdateRelease.MAX_APK+1),manifest(new byte[]{1}).put("sha256","bad"),manifest(new byte[]{1}).put("versionName","bad"),manifest(new byte[]{1}).put("versionCode",-1)}){
            try{new UpdateRelease(j);fail();}catch(IOException expected){}
        }
    }
    @Test public void streamsAndVerifies()throws Exception{
        byte[] bytes=new byte[150000];new java.util.Random(1).nextBytes(bytes);UpdateRelease r=new UpdateRelease(manifest(bytes));ByteArrayOutputStream out=new ByteArrayOutputStream();long[] progress={0};
        r.copyVerified(new ByteArrayInputStream(bytes),out,(done,total)->{progress[0]=done;assertEquals(bytes.length,total);});assertArrayEquals(bytes,out.toByteArray());assertEquals(bytes.length,progress[0]);
    }
    @Test public void rejectsTamperedTruncatedAndOversizedBytes()throws Exception{
        UpdateRelease r=new UpdateRelease(manifest(new byte[]{1,2,3}));
        for(byte[] b:new byte[][]{{1,2,4},{1,2},{1,2,3,4}}){try{r.copyVerified(new ByteArrayInputStream(b),new ByteArrayOutputStream(),(d,t)->{});fail();}catch(IOException expected){}}
    }
    @Test public void cancellationStopsTransfer()throws Exception{
        UpdateRelease r=new UpdateRelease(manifest(new byte[]{1,2,3}));try{r.copyVerified(new ByteArrayInputStream(new byte[]{1,2,3}),new ByteArrayOutputStream(),(d,t)->{throw new IOException("cancel");});fail();}catch(IOException expected){assertEquals("cancel",expected.getMessage());}
    }
}
