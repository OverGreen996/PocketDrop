package local.pocketdrop.android;

import java.net.URI;
import java.util.Locale;

/** Keep QR and mDNS from redirecting credentials outside a private IPv4 LAN. */
public final class LanRules {
    private LanRules() {}
    public static boolean privateHost(String host) {
        if (host == null || !host.matches("[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+")) return false;
        String[] parts = host.split("\\."); int[] n = new int[4];
        try { for (int i=0;i<4;i++) { n[i]=Integer.parseInt(parts[i]); if(n[i]>255 || !parts[i].equals(Integer.toString(n[i]))) return false; } } catch (NumberFormatException e) { return false; }
        return n[0]==10 || (n[0]==172 && n[1]>=16 && n[1]<=31) || (n[0]==192 && n[1]==168) || (n[0]==169 && n[1]==254);
    }
    public static String endpoint(String raw) throws Exception {
        URI u=new URI(raw);
        if (!"https".equals(u.getScheme()) || !privateHost(u.getHost()) || u.getPort()<1 || u.getPort()>65535 || u.getRawUserInfo()!=null || u.getRawQuery()!=null || u.getRawFragment()!=null || !(u.getRawPath().isEmpty() || "/".equals(u.getRawPath()))) throw new IllegalArgumentException("配對資料必須指向同一區域網路");
        return "https://"+u.getHost()+":"+u.getPort();
    }
    public static boolean safeName(String name) {
        if (name==null || name.isEmpty() || name.codePointCount(0,name.length())>180 || name.endsWith(".") || name.endsWith(" ") || name.chars().anyMatch(c -> Character.isISOControl(c) || "\\/:*?\"<>|".indexOf(c)>=0)) return false;
        String stem=name.split("\\.",2)[0].toUpperCase(Locale.ROOT);
        return !stem.matches("CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9]");
    }
}
