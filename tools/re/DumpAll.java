import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.io.*;
public class DumpAll extends GhidraScript {
  public void run() throws Exception {
    String out = getScriptArgs()[0];
    PrintWriter sym = new PrintWriter(new FileWriter(out+"/symbols.tsv"));
    for (Symbol s : currentProgram.getSymbolTable().getAllSymbols(true)) {
      if (s.getSource()==SourceType.DEFAULT) continue;
      sym.println(s.getAddress()+"\t"+s.getSymbolType()+"\t"+s.getName(true));
    }
    sym.close();
    DecompInterface di = new DecompInterface();
    di.toggleCCode(true);
    di.openProgram(currentProgram);
    PrintWriter pw = new PrintWriter(new FileWriter(out+"/decomp.c"));
    int n=0;
    for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
      if (monitor.isCancelled()) break;
      if (f.isThunk() || f.isExternal()) continue;
      DecompileResults r = di.decompileFunction(f, 60, monitor);
      pw.println("//==== FUNC "+f.getEntryPoint()+" "+f.getName(true)+" size="+f.getBody().getNumAddresses());
      if (r!=null && r.decompileCompleted()) pw.println(r.getDecompiledFunction().getC());
      else pw.println("// decompile failed");
      if (++n % 500 == 0) { pw.flush(); println("done "+n); }
    }
    pw.close();
  }
}
