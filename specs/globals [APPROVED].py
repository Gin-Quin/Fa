
# en Fa, les variables sont retrouvées dans le scope le plus proche
# un GET d'une variable qui n'est pas retrouvée créé une erreur du compilateur Fa
# un SET d'une variable qui n'est pas retrouvée créé la variable dans le plus petit scope

# Les variables globales sont retrouvées / définies grâce au mot-clé global

global x = 12  -- we define a global variable
global x : String  -- we define a global variable
global z  -- we use a global variable

# When the Fa compiler encounters the use of a global variable, it checks it has previously been declared in another module

# The fact globals must be declared in any file which use them make sure you don't rewrite/use a global without knowing about it