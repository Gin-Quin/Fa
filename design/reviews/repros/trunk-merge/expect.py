# Replaces the `-- prints:` annotations of a repro with the given lines (stdin),
# in order: the inline ones are rewritten, extra lines are appended.
import re, sys
path = sys.argv[1]
lines = sys.stdin.read().splitlines()
text = open(path).read().splitlines()
out = []
for line in text:
    match = re.search(r'-- prints: .*$', line)
    if match and lines:
        line = line[:match.start()] + '-- prints: ' + lines.pop(0)
    elif match:
        line = line[:match.start()].rstrip()
    out.append(line)
out += ['-- prints: ' + line for line in lines]
open(path, 'w').write('\n'.join(out) + '\n')
