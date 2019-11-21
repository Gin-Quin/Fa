# Fa can read globs instead of regular expressions
# Globs are especially useful to match files and routes
# The whole Fa FileSystem functions which take Strings as input actually take globs.
# This allow to do super-sweet stuff like :

for file in FileSystem |subs/**/*.txt|
	print file