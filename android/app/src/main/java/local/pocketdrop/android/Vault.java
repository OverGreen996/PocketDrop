package local.pocketdrop.android;

import android.content.Context;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import org.json.JSONObject;
import java.security.KeyStore;
import java.security.KeyPairGenerator;
import java.security.spec.ECGenParameterSpec;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

final class Vault {
    private final Context context;
    Vault(Context c) { context=c.getApplicationContext(); }
    private KeyStore store() throws Exception { KeyStore s=KeyStore.getInstance("AndroidKeyStore");s.load(null);return s; }
    private SecretKey key() throws Exception {
        KeyStore s=store();
        if(!s.containsAlias("pocketdrop-vault")) { KeyGenerator g=KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore");g.init(new KeyGenParameterSpec.Builder("pocketdrop-vault",KeyProperties.PURPOSE_ENCRYPT|KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());g.generateKey(); }
        return (SecretKey)store().getKey("pocketdrop-vault",null);
    }
    String deviceId() {
        var p=context.getSharedPreferences("identity",0);String id=p.getString("id",null);
        if(id==null){id=UUID.randomUUID().toString();p.edit().putString("id",id).apply();}return id;
    }
    String publicKey() throws Exception {
        if(!store().containsAlias("pocketdrop-identity")){KeyPairGenerator g=KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC,"AndroidKeyStore");g.initialize(new KeyGenParameterSpec.Builder("pocketdrop-identity",KeyProperties.PURPOSE_SIGN|KeyProperties.PURPOSE_VERIFY).setAlgorithmParameterSpec(new ECGenParameterSpec("secp256r1")).setDigests(KeyProperties.DIGEST_SHA256).build());g.generateKeyPair();}
        return Base64.encodeToString(store().getCertificate("pocketdrop-identity").getPublicKey().getEncoded(),Base64.NO_WRAP);
    }
    void save(JSONObject data) throws Exception {
        Cipher c=Cipher.getInstance("AES/GCM/NoPadding");c.init(Cipher.ENCRYPT_MODE,key());
        String value=Base64.encodeToString(c.getIV(),Base64.NO_WRAP)+":"+Base64.encodeToString(c.doFinal(data.toString().getBytes(StandardCharsets.UTF_8)),Base64.NO_WRAP);
        if(!context.getSharedPreferences("vault",0).edit().putString("paired",value).commit()) throw new IllegalStateException("無法保存配對");
    }
    JSONObject load() throws Exception {
        String value=context.getSharedPreferences("vault",0).getString("paired",null);if(value==null)return null;
        String[] parts=value.split(":",2);Cipher c=Cipher.getInstance("AES/GCM/NoPadding");c.init(Cipher.DECRYPT_MODE,key(),new GCMParameterSpec(128,Base64.decode(parts[0],Base64.NO_WRAP)));
        return new JSONObject(new String(c.doFinal(Base64.decode(parts[1],Base64.NO_WRAP)),StandardCharsets.UTF_8));
    }
    void forget(){context.getSharedPreferences("vault",0).edit().remove("paired").apply();}
}
