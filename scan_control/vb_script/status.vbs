Set s = CreateObject("Scan.Application")
stat = s.StatusText
stat_id = s.StatusId
WScript.StdOut.WriteLine stat_id
WScript.StdOut.WriteLine stat