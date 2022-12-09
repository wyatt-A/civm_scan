Set s = CreateObject("Scan.Application")
ppr = WScript.Arguments(0)
s.PPR = ppr
WScript.StdOut.Write "ppr:"
WScript.StdOut.WriteLine s.PPR