Set s = CreateObject("Scan.Application")
stat = s.StatusText
stat_id = s.StatusId
WScript.StdOut.Write "status_id:"
WScript.StdOut.WriteLine stat_id
WScript.StdOut.Write "status_text:"
WScript.StdOut.WriteLine stat