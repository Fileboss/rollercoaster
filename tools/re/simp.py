import re,sys
s=sys.stdin.read()
# remove mode-check throw blocks
s=re.sub(r'\n\s*if \(\(~?\*\(u?(?:int|byte) \*\)\((?:param_\d|\w+) \+ (?:4|0x20)\) & 1\) [=!]= 0\) \{\n\s*\w+ = FUN_14000df50\([^;]*\);\n\s*/\* WARNING: Subroutine does not return \*/\n\s*AfxThrowArchiveException\(\d,\w+\);\n\s*\}', '', s)
s=re.sub(r'\n\s*\w+ = ARNAME;\n\s*/\* WARNING: Subroutine does not return \*/\n\s*AfxThrowArchiveException\(\d,\w+\);', '\n<<THROW>>', s)
s=re.sub(r'\n\s*if \(\(~?\*\(u?(?:int|byte) \*\)\((?:param_\d|\w+) \+ (?:4|0x20)\) & 1\) [=!]= 0\) \{\n<<THROW>>\n\s*\}', '', s)
s=re.sub(r'\n\s*if \(\(ulonglong\)\w+\[8\] < [^\n]*\) \{\n\s*AR_(?:Fill|Flush)\([^\n]*\);\n\s*\}', '', s)
s=re.sub(r'\n\s*if \(\*\(ulonglong \*\)\(\w+ \+ 0x40\) < [^\n]*\) \{\n\s*AR_(?:Fill|Flush)\([^\n]*\n?[^\n]*\);\n\s*\}', '', s)
print(s)
