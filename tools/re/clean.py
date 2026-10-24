import re,sys
s=sys.stdin.read()
# refcount release blocks
s=re.sub(r'\n[ \t]*LOCK\(\);\n[ \t]*\w+ = \([^;]*\);\n[ \t]*(\w+) = \*\w+;\n[ \t]*\*\w+ = \*\w+ \+ -1;\n[ \t]*UNLOCK\(\);\n[ \t]*if \(\(?(?:int\))?\(?\1\)? < 2\) \{\n[^\n]*\n[ \t]*\}', '', s)
s=re.sub(r'\n[ \t]*LOCK\(\);\n[ \t]*\w+ = \([^;]*\);\n[ \t]*(\w+) = \*\w+;\n[ \t]*\*\(int \*\)\w+ = \(int\)\*\w+ \+ -1;\n[ \t]*UNLOCK\(\);\n[ \t]*if \(\(int\)\1 < 2\) \{\n[^\n]*\n[ \t]*\}', '', s)
# string manager alloc
s=re.sub(r'\n([ \t]*)(\w+) = \(longlong \*\)FUN_1401258a8\(\);\n[ \t]*if \(\2 == \(longlong \*\)0x0\) \{\n[ \t]*/\* WARNING: Subroutine does not return \*/\n[ \t]*FUN_14000e660\(0x80004005\);\n[ \t]*\}\n[ \t]*(\S+) = \(\*\*\(code \*\*\)\(\*\2 \+ 0x18\)\)\(\2\);\n[ \t]*\3 = \3 \+ 0x18;', r'\n\1\3 = CString_new();', s)
names={'14000e0f0':'CStr_AssignSz','14000e210':'CStr_Release','14000e060':'CStr_Assign','14000e810':'CStr_CtorW',
'14000e680':'CStr_SetString','14000da00':'CStr_EqualsSz','14000db10':'CStr_Concat','14000dc10':'CStr_ConcatRaw',
'14000ded0':'CStr_CtorSz','14000dd20':'CStr_AppendRaw','1400130a0':'CStr_FromInt','14000df70':'CStrData_Clone',
'1401258a8':'AfxStrMgr','14000e660':'AtlThrow','14000e2d0':'CStr_LoadRes','14013038c':'operator_new','140338f00':'memset','140338860':'memcpy',
'14014b864':'CPtrList_AddTail','14014b8a8':'CPtrList_Find','1400773a0':'GetMainFrame'}
for k,v in names.items(): s=s.replace('FUN_'+k,v)
print(s)
