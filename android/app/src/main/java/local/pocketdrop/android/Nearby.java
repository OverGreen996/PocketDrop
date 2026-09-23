package local.pocketdrop.android;

import android.content.Context;
import android.net.nsd.NsdManager;
import android.net.nsd.NsdServiceInfo;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import java.net.InetAddress;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.function.Consumer;

/** Every callback belongs to one foreground scan; old scans cannot stop a new one. */
final class Nearby {
    private final NsdManager manager;
    private final WifiManager.MulticastLock multicast;
    private final Handler main=new Handler(Looper.getMainLooper());
    private Scan active;
    Nearby(Context context){manager=(NsdManager)context.getSystemService(Context.NSD_SERVICE);WifiManager wifi=(WifiManager)context.getApplicationContext().getSystemService(Context.WIFI_SERVICE);multicast=wifi.createMulticastLock("PocketDrop nearby");multicast.setReferenceCounted(false);}
    void start(String deviceId,Consumer<String> found){stop();Scan scan=new Scan(deviceId,found);active=scan;try{multicast.acquire();manager.discoverServices("_pocketdrop._tcp.",NsdManager.PROTOCOL_DNS_SD,scan);}catch(RuntimeException e){stop();}}
    void stop(){Scan old=active;active=null;if(old!=null){try{manager.stopServiceDiscovery(old);}catch(RuntimeException ignored){}if(Build.VERSION.SDK_INT>=34)old.unwatch();}if(multicast.isHeld())multicast.release();}
    private final class Scan implements NsdManager.DiscoveryListener {
        final String deviceId;final Consumer<String> found;
        final ArrayDeque<NsdServiceInfo> queue=new ArrayDeque<>();
        final HashSet<String> names=new HashSet<>();
        final ArrayList<NsdManager.ServiceInfoCallback> watches=new ArrayList<>();
        boolean resolving;int retries;
        Scan(String id,Consumer<String> callback){deviceId=id;found=callback;}
        boolean current(){return active==this;}
        public void onDiscoveryStarted(String s){}
        public void onDiscoveryStopped(String s){}
        public void onStartDiscoveryFailed(String s,int error){main.post(()->{if(current())stop();});}
        public void onStopDiscoveryFailed(String s,int error){}
        public void onServiceLost(NsdServiceInfo service){main.post(()->{if(current())names.remove(service.getServiceName());});}
        public void onServiceFound(NsdServiceInfo service){main.post(()->{
            if(!current()||!service.getServiceName().startsWith("PocketDrop-")||!names.add(service.getServiceName()))return;
            if(Build.VERSION.SDK_INT>=34)watch(service);else{queue.add(service);resolveNext();}
        });}
        @android.annotation.TargetApi(34) void watch(NsdServiceInfo service){
            NsdManager.ServiceInfoCallback callback=new NsdManager.ServiceInfoCallback(){
                public void onServiceInfoCallbackRegistrationFailed(int errorCode){main.post(()->{if(current()){names.remove(service.getServiceName());watches.remove(this);}});}
                public void onServiceUpdated(NsdServiceInfo info){if(current())publish(info);}
                public void onServiceLost(){}
                public void onServiceInfoCallbackUnregistered(){}
            };
            watches.add(callback);
            try{manager.registerServiceInfoCallback(service,main::post,callback);}catch(RuntimeException e){watches.remove(callback);names.remove(service.getServiceName());}
        }
        @android.annotation.TargetApi(34) void unwatch(){for(var callback:watches)try{manager.unregisterServiceInfoCallback(callback);}catch(RuntimeException ignored){}watches.clear();}
        @SuppressWarnings("deprecation") void resolveNext(){
            if(!current()||resolving||queue.isEmpty())return;
            resolving=true;NsdServiceInfo service=queue.peek();
            try{manager.resolveService(service,new NsdManager.ResolveListener(){
                public void onResolveFailed(NsdServiceInfo s,int error){main.post(()->{
                    if(!current())return;resolving=false;
                    if(++retries>=3){queue.poll();retries=0;}
                    main.postDelayed(()->resolveNext(),500);
                });}
                public void onServiceResolved(NsdServiceInfo s){main.post(()->{
                    if(!current())return;resolving=false;queue.poll();retries=0;publish(s);resolveNext();
                });}
            });}catch(RuntimeException e){resolving=false;queue.poll();main.postDelayed(()->resolveNext(),500);}
        }
        @SuppressWarnings("deprecation") void publish(NsdServiceInfo info){
            if(!current())return;byte[] id=info.getAttributes().get("id");
            if(id==null||(deviceId!=null&&!deviceId.equals(new String(id,StandardCharsets.UTF_8))))return;
            List<InetAddress> hosts=Build.VERSION.SDK_INT>=34?info.getHostAddresses():info.getHost()==null?Collections.emptyList():Collections.singletonList(info.getHost());
            for(InetAddress address:hosts){String host=address.getHostAddress();if(LanRules.privateHost(host)&&info.getPort()>0)found.accept((deviceId==null?new String(id,StandardCharsets.UTF_8)+"|":"")+"https://"+host+":"+info.getPort());}
        }
    }
}
