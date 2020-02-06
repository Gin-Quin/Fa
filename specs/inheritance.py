# Réfléchir au problème de l'héritage multiple


# Il est possible de faire de l'héritage multiple en Javascript à l'aide de la technique suivante :
# Supposons les héritages suivants :
class A
class B is A
class X
class Y is X

# Maintenant supposons qu'on souhaite créer la classe C telle que :
class C is B, Y

# Alors le hack est de créer un clone du prototype de C :
