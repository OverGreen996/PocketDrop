package local.pocketdrop.android;

import java.util.*;

/** Monotonic retry schedule, independent of whether NSD repeats a callback. */
final class EndpointCandidates {
    private static final long RETRY_MS=5000, TTL_MS=120000;
    private static final class Entry {long seen,attempt=Long.MIN_VALUE;Entry(long now){seen=now;}}
    private final LinkedHashMap<String,Entry> entries=new LinkedHashMap<>();
    void clear(){entries.clear();}
    void observe(String endpoint,long now){
        try{endpoint=LanRules.endpoint(endpoint);}catch(Exception e){throw new IllegalArgumentException("Invalid LAN endpoint",e);}
        Entry existing=entries.get(endpoint);
        if(existing!=null){existing.seen=now;return;}
        if(entries.size()>=32)entries.remove(entries.keySet().iterator().next());
        entries.put(endpoint,new Entry(now));
    }
    List<String> due(long now,int limit,Set<String> inFlight){
        entries.entrySet().removeIf(e->now-e.getValue().seen>TTL_MS);
        ArrayList<Map.Entry<String,Entry>> sorted=new ArrayList<>(entries.entrySet());
        sorted.sort((a,b)->Long.compare(b.getValue().seen,a.getValue().seen));
        ArrayList<String> result=new ArrayList<>();
        for(var item:sorted){if(result.size()>=limit)break;Entry e=item.getValue();
            if(!inFlight.contains(item.getKey())&&(e.attempt==Long.MIN_VALUE||now-e.attempt>=RETRY_MS)){
                e.attempt=now;result.add(item.getKey());
            }
        }
        return result;
    }
}
