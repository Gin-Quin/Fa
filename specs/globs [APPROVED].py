# Fa can read globs instead of regular expressions
# Globs are especially useful to match files and routes
# This allow to do super-sweet stuff like :

for file in (|subs/**/*.txt|fs)  # 'fs' is the option
	print file
