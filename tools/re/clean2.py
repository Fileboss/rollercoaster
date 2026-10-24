import re,sys
s=sys.stdin.read()
s=re.sub(r'/\* WARNING: Removing unreachable block[^\n]*\n','',s)
# collapse alloc idiom with goto
s=re.sub(r'\n([ \t]*)(\w+) = \(longlong \*\)AfxStrMgr\(\);\n[ \t]*if \(\2 == \(longlong \*\)0x0\) (?:goto \w+;|\{\n[^\n]*\n[^\n]*\n[ \t]*\})\n[ \t]*(\S+) = \(\*\*\(code \*\*\)\(\*\2 \+ 0x18\)\)\(\2\);\n[ \t]*\3 = \3 \+ 0x18;', r'\n\1\3 = CString_new();', s)
s=re.sub(r'\n([ \t]*)(\w+) = \(longlong \*\)AfxStrMgr\(\);\n[ \t]*if \(\2 == \(longlong \*\)0x0\) (?:goto \w+;|\{\n[^\n]*\n[^\n]*\n[ \t]*\})\n[ \t]*(\S+) = \(\*\*\(code \*\*\)\(\*\2 \+ 0x18\)\)\(\2\);\n[ \t]*(\S+) = \3 \+ 0x18;', r'\n\1\4 = CString_new();', s)
# remove decls
s=re.sub(r'\n  (undefined\d?|longlong|int|uint|char|bool|ulonglong|ushort|short|code|byte|float|LPCWSTR|HDC|CString)[ *]+\w+(\s*\[\d+\])?;','',s)
# remove release blocks variants with lVar = *(plVar+...) patterns
s=re.sub(r'\n[ \t]*LOCK\(\);\n(?:[^\n]*\n){1,4}?[ \t]*UNLOCK\(\);\n[ \t]*if \([^\n]*< 2\) \{\n[^\n]*\n[ \t]*\}','',s)
s=re.sub(r'\n[ \t]*LOCK\(\);\n(?:[^\n]*\n){1,4}?[ \t]*UNLOCK\(\);','',s)
s=re.sub(r'\n[ \t]*CStr_Release\([^;]*\);','',s)
s=re.sub(r'\n[ \t]*/\* WARNING: Subroutine does not return \*/\n[ \t]*AtlThrow\(0x80004005\);','\n THROW;',s)
print(s)
