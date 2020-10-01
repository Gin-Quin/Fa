# The file as object is great for fast developing
# The idea is simple : the file describes a single object
# Fa's compiler detect if the file is a regulard Fa's file or an object depending on the file extension :
# A file object has the format : `{filename}.{inherited class}.fa`
# For exemple the file `cesar.dog.fa` would represent a `Dog` object. The `Dog` class definition must be findable by the compiler - either as a superglobal class or because the file 'Dog.fa' is in the same folder.
