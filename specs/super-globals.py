
# Super globals are variables that are accessible to all source files 
# without even needing to declare them

# There can be three types of superglobals :
# - superglobals defined by the standard library
# - superglobals defined by a third-party library - typically a framework
# - superglobals defined by the user

# I will not discuss here whether the use of superglobals is bad or not

# I personnally think : every project should have its own and unique superglobal
# (aka its store)