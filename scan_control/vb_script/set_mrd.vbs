Set s = CreateObject("Scan.Application")
mrd = WScript.Arguments(0)
s.Output = mrd
WScript.StdOut.WriteLine s.Output