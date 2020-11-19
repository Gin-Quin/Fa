# Réfléchir au problème de l'héritage multiple
# -> Après réflexion, je pense que l'héritage multiple est une mauvaise idée
# Le concept est cool mais :
# 1. Peut mener à des cas tordus et ambigüs (ex: the dreaded diamond)
# 2. N'existe pas nativement en Javascript
# 3. Est agréablement remplaçable par de la composition / programmation orientée entités

# Il est possible de faire de l'héritage multiple en Javascript à l'aide de la technique suivante :
# Supposons les héritages suivants :
class A
class B is A
class X
class Y is X

# Maintenant supposons qu'on souhaite créer la classe C telle que :
class C is B, Y

# Alors le hack est de créer un clone du prototype de C :
