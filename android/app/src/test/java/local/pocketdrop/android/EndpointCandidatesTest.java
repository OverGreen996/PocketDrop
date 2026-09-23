package local.pocketdrop.android;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.*;

public class EndpointCandidatesTest {
 @Test public void retriesWithoutAnotherNsdCallback(){
  EndpointCandidates c=new EndpointCandidates();String endpoint="https://192.168.1.2:32000";
  c.observe(endpoint,0);assertEquals(List.of(endpoint),c.due(0,2,Set.of()));
  assertTrue(c.due(1000,2,Set.of()).isEmpty());
  assertEquals(List.of(endpoint),c.due(5000,2,Set.of()));
  assertEquals(List.of(endpoint),c.due(10000,2,Set.of()));
 }
 @Test public void limitsWorkAndPrefersFreshPort(){
  EndpointCandidates c=new EndpointCandidates();String old="https://192.168.1.2:32000",next="https://192.168.1.2:32001";
  c.observe(old,0);c.observe(next,1);assertEquals(List.of(next),c.due(1,1,Set.of(old)));
  assertTrue(c.due(2,0,Set.of()).isEmpty());assertTrue(c.due(120002,2,Set.of()).isEmpty());
  c.observe(next,130000);c.clear();assertTrue(c.due(130001,2,Set.of()).isEmpty());
 }
 @Test public void neverProbesInternetCandidates(){
  try{new EndpointCandidates().observe("https://8.8.8.8:443",0);fail("public endpoint accepted");}catch(IllegalArgumentException expected){}
 }
}
