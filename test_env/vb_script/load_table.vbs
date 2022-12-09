Set s = CreateObject("Scan.Application")
ascii_table = WScript.Arguments(0)
success = s.LoadUpperMemory(ascii_table)
WScript.StdOut.WriteLine success