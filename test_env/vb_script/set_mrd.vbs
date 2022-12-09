Set s = CreateObject("Scan.Application")
s.PromptOverWriteMRDfile = 0
mrd = WScript.Arguments(0)
s.Output = mrd
WScript.StdOut.WriteLine s.Output