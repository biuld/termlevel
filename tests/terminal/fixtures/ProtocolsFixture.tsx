import { Clipboard, cleanup, mount, signal, Text, View } from "nsh";
import { Button } from "nsh/base";
import { NativeInputProbe, Runtime, Sessions, SessionTerminal } from "termlevel/native";
function Fixture() {
  const session=Sessions.open(Runtime.cwd(),"#eeeeee","#202020","/bin/sh");
  const error=signal("");
  const clipboard=signal("");
  cleanup(()=>{Sessions.setClipboardRead(false);Sessions.close(session.id);});
  return <View class="flex-1 bg-[#202020]">
    <SessionTerminal testID="protocol-terminal" sessionId={session.id} class="min-h-0 flex-1" fontSize={14} lineHeight={22} defaultForeground="#eeeeee" defaultBackground="#202020" />
    <Text testID="clipboard-value">{`CLIP_${clipboard()}`}</Text>
    <View class="flex-row">
    <Button testID="clipboard-allow" onPress={()=>Sessions.setClipboardRead(true)}><Text>Allow read</Text></Button>
    <Button testID="clipboard-save" onPress={async()=>{await NativeInputProbe.pasteboard("save");}}><Text>Save clipboard</Text></Button>
    <Button testID="clipboard-system-read" onPress={async()=>clipboard.set(await NativeInputProbe.pasteboard("read"))}><Text>Read system clipboard</Text></Button>
    <Button testID="clipboard-read" onPress={async()=>clipboard.set(await Clipboard.readText())}><Text>Read clipboard</Text></Button>
    <Button testID="clipboard-restore" onPress={async()=>{await NativeInputProbe.pasteboard("restore");}}><Text>Restore clipboard</Text></Button>
    </View>
    <Text testID="protocol-error">{error()}</Text>
    <Button testID="native-key-sequence" onPress={async ()=>{
      try {NativeInputProbe.syntheticKeys(false);NativeInputProbe.physicalKeys(true);await new Promise(resolve=>setTimeout(resolve,80));for(const [code,text] of [[0,"a"],[36,"\r"]] as const) for(const action of ["press","repeat","release"]) {await NativeInputProbe.key(action,code,text,0);}}
      catch(e){error.set(String(e));}
    }}><Text>Native keys</Text></Button>
  </View>;
}
mount(Fixture);
