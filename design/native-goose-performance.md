# Rapport intermédiaire : performances natives de Fa face à Goose

Analyse du 3 novembre 2026, branche `perf/native-goose`, départ
`9c3032aa3eb4d8dced7cb94f7aeb99d0624e6a9c`.
Dernier code mesuré : `6067418dbe9db284956377b75d2d653b41d9712f` ; pour
`string-building`, `86b3c1ca75988387b8ba710a4c40a1fe05730ae8` (voir sa section).
Correctifs de durée de vie validés : `266aa15faac0d820558b6ecdadf2c97f21033e24`.
Les 25 exécutables reconstruits à cette révision sont identiques octet par octet
aux exécutables mesurés ; leurs sorties ont été vérifiées à nouveau.

## Résultat et critère d'acceptation

**L'objectif « Fa au moins aussi rapide que Goose sur chacun des 25 benchmarks »
n'est pas atteint.** Les changements de cette branche réduisent plusieurs surcoûts
du compilateur, mais les unions boxées, certaines chaînes et les tables possèdent
encore des coûts importants. Ce document distingue les
améliorations vérifiées du travail restant ; il ne constitue pas une validation de
la minimalité de tout le code généré.

Les sources des benchmarks Fa, Goose et Rust sont conservées telles quelles.
Les résultats des programmes sont comparés octet par octet avec ceux de Rust
avant toute mesure. Aucun test de sécurité non prouvé n'est désactivé pour
améliorer les chiffres.

## Protocole reproductible

- Machine : Linux x86-64, VM KVM, 6 cœurs, AMD EPYC Processor (with IBPB).
- Fa : compilateur Rust en release, LLVM 18, `--target=native --emit-ir`,
  optimisation habituelle O3, vérifications de débordement désactivées comme
  dans le benchmark existant. Les autres vérifications restent actives.
- Goose : `eadca40`, runtime `3025e5810379`, `build -O2 --standalone`,
  clang 18.1.3, même architecture générique que Fa.
- Référence de correction : `rustc 1.99.1`, `--edition 2024 -C opt-level=3`.
- Instructions : Callgrind `Ir`, total dynamique du processus, runtime et
  bibliothèques inclus. Il s'agit d'instructions machine exécutées, pas du nombre
  de lignes LLVM. Les petites variations du démarrage ne sont pas significatives.
- Temps : mesures alternées Fa/Goose, après vérification et échauffement ; médiane
  du temps CPU utilisateur + système du processus, avec les temps muraux et tous
  les échantillons conservés dans le JSON. La VM est partagée : les petits écarts
  temporels ne suffisent pas à conclure à une supériorité.
- Deux échelles : tailles d'audit pour le profilage, tailles par défaut de la
  suite pour les temps. Les tailles exactes figurent dans les résultats.
- Dernier relevé : sept échantillons par langage aux tailles par défaut, processus
  et descendants fixés au CPU logique 1, sans autre compilation de cette tâche.
  Les temps recueillis en parallèle des tests pendant l'exploration ne servent
  pas au tableau final.

Le nouveau pilote `benchmarks/audit/compare.py` conserve les exécutables, sorties,
empreintes SHA-256, LLVM optimisé de Fa, C et LLVM O2 de Goose lorsque `GOOSE_CC`
est clang, assembleur et profils. `ir_counts.py` compte séparément les opérations
LLVM **statiques**, par fonction. `hot.py` attribue les instructions exécutées à
l'assembleur. Les artefacts volumineux sont ignorés par Git ; un relevé compact
est conservé à côté de ce rapport.

```sh
export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"
export GOOSE="$HOME/.local/goose/goose"
export GOOSE_CC="$LLVM_SYS_181_PREFIX/bin/clang"
cargo build --release -p fa

# Avant de modifier le compilateur : garder ce répertoire intact.
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before --runs 9
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/before \
  --reuse --profile --runs 1 --report profiles.json

# Après les modifications : nouveaux exécutables et nouvelle vérification Rust.
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/after --runs 9
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/after \
  --reuse --profile --runs 1 --report profiles.json
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/default \
  --default-sizes --cpu 1 --runs 7

# Pour refaire seulement Fa en gardant exactement le même binaire Goose :
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/after \
  --reuse --rebuild-fa --profile --runs 1 --report profiles.json

python3 benchmarks/audit/ir_counts.py benchmarks/audit/build/after/scene/fa/native/main.ll
python3 benchmarks/audit/hot.py benchmarks/audit/build/after/calc/fa/callgrind.out \
  benchmarks/audit/build/after/calc/fa/native/main.s --calls
```

`VALGRIND` peut désigner le binaire réel de Valgrind, et `VALGRIND_LIB` son
répertoire de bibliothèques. `GOOSE_HOME` permet d'isoler son cache de compilation.

## Changements et vérification du LLVM

### Constantes globales

Un `let` global de valeur scalaire littérale ou déjà calculée à la compilation
est initialisé dans une globale LLVM `constant`, au lieu d'une globale mutable
remplie dans `main`. Les alphabets et les constantes numériques peuvent ainsi
être propagés dans les boucles et les fonctions. Seuls les initialiseurs sans
évaluation runtime sont éligibles ; les appels arbitraires et les valeurs
mutables ne le sont pas.

Le test d'IR vérifie que l'indexation d'un alphabet global constant n'a plus de
lecture de `fa.global` après optimisation.

La longueur d'un tableau littéral à travers des bindings immuables devient aussi
une constante. L'évaluation du tableau et de ses initialiseurs reste exécutée.
Les `mutable`, les références `use`, les spreads et les formes non prouvées
gardent leurs lectures ordinaires. Cela supprime notamment des divisions par une
longueur chargée dynamiquement, et les gardes que LLVM peut démontrer inutiles.

### Paramètres agrégés

Les appels directs synchrones empruntent les objets, chaînes, tuples et autres
agrégats sans paramètre par défaut. Un objet de même représentation est passé à
son adresse, sans copie de ses champs. Une vue structurelle de disposition
différente conserve une adaptation, mais ne réécrit pas l'argument en lecture
seule. Les expressions temporaires gardent leur durée de vie pendant l'appel.

Exemple concret : `scene.sumWorld` recevait une structure de 112 octets par valeur.
Son LLVM reçoit désormais `ptr nocapture readonly` ; les charges et sauvegardes
de toute la structure à chaque appel récursif disparaissent. Le `readonly` est
inféré par LLVM, sans assertion `noalias` ajoutée sans preuve.

Les fonctions indirectes gardent leur convention existante avec un adaptateur
pour les fonctions nommées. Les coroutines et les paramètres par défaut gardent
leur convention actuelle : le changement ne résout donc pas encore toutes les
copies de paramètres du langage. Les tests couvrent les lambdas, les destructeurs,
les vues structurelles, les temporaires et le suivi des lectures réactives.

Les constantes `static` et les constructeurs de membres sans données sont des
valeurs calculées, sans slot runtime à emprunter. Ils passent par un temporaire
de la durée de l'appel ; les variables stockées gardent leur passage par adresse.

### Optionnels récursifs

Une arête récursive optionnelle et une union boxée optionnelle utilisent un
pointeur nullable. Un nœud de `binary-trees` contient deux pointeurs, soit 16
octets au lieu de deux paires `{ i1, ptr }` occupant 32 octets. L'absence est
`null`, la présence est le pointeur lui-même. Copies, destructions, motifs,
chaînes optionnelles et réception par canal utilisent la même représentation.

La preuve est structurelle : une boîte présente n'est jamais nulle. Cela
n'autorise pas à supprimer le test d'absence lorsqu'un programme demande le
contenu d'un optionnel. Un test ajouté a aussi découvert et corrigé un déballage
erroné d'un optionnel dont la représentation ne changeait pas.

### Allocation et copies de chaînes

Le chemin rapide de l'allocateur inclut maintenant le bump d'une page de petits
objets, en plus de la liste de blocs libres. Le calcul de classe et de pas se
spécialise pour une taille constante ; seul le remplissage d'une page passe par
le chemin lent. Les calculs de fin de page évitent une addition de pointeur hors
allocation. Une allocation de taille zéro demandant un alignement de 16 conserve
cet alignement.

La copie d'une chaîne statique ou vide est inlinée. L'allocation et la copie des
octets vivent dans une fonction séparée : le chemin courant d'un caractère ASCII
statique ne paie plus les appels et sauvegardes de registres du chemin possédant
de la mémoire. Une longue chaîne possédée qui doit être copiée reste copiée.

Le retour de `fa_alloc` est marqué `noalias` et `nonnull` : un bloc réussi est
frais et ne recouvre aucun bloc vivant ; en cas d'échec le runtime termine le
programme. Cette propriété concerne le résultat de l'allocateur. Elle n'ajoute
aucune hypothèse de non-aliasing entre les paramètres des fonctions Fa.

Les chaînes courtes stockent désormais leurs octets directement dans les trois
mots de la valeur : 23 octets sur les cibles 64 bits, 11 sur wasm32. Le dernier
octet contient les drapeaux et la longueur. Les octets inutilisés sont mis à zéro,
ce qui permet une égalité par mots et un hash sans copie vers un buffer temporaire.
Les copies courtes ne font aucune allocation. Une longue chaîne garde son buffer
possédé à croissance géométrique ; les capacités excluent les bits réservés aux
drapeaux. Les vues sont matérialisées seulement lorsqu'une copie possédée est requise.

### Petites unions non récursives

Un petit payload fini composé de scalaires, chaînes et objets sans destructeur
peut rester dans la valeur avec son tag, sans boîte. Le budget est de trois mots
machine pour le payload, comme le descripteur d'une chaîne. La convention de
copie, les optionnels, les fonctions indirectes et les conversions vers une
union générale sont couverts par les tests. Une union ne possédant aucune donnée
à détruire ne génère pas de parcours de destruction par élément.

Lors d'une conversion vers une union boxée, un membre sans données utilise une
boîte statique. Le chemin des membres avec données copie leur payload possédé.
Une vue créée sur la pile ne peut donc pas s'échapper via le raccourci de copie
des membres sans données ; ce cas est aussi vérifié sans optimisation.

Les octets de padding sont initialisés explicitement lors de la construction.
Un `freeze` protège les représentations par mots pouvant avoir été réécrites
via une vue du payload : le padding d'un store agrégé peut être indéfini en LLVM.
Ce traitement est local aux valeurs contenant une union inline. Voir la
[référence LLVM sur les stores](https://llvm.org/docs/LangRef.html#store-instruction)
et sur [les valeurs indéfinies](https://llvm.org/docs/UndefinedBehavior.html).

**L'inlining systématique des plus grosses unions a été écarté.** L'essai à
40 octets par élément de `Array<Event>` réduisait les instructions de `records`
mais élargissait le tableau. Une comparaison directe, neuf échantillons alternés
sur le CPU 1, a mesuré 0,433 s pour le code précédent et 0,605 s pour cet essai,
contre 0,196 s pour Goose. Le binaire précédent a été reconstruit depuis
`f9de4a6` et son SHA-256 correspond exactement à celui du relevé initial.
Ce chemin expérimental n'est pas inclus dans les résultats finaux. Les unions
plus grosses gardent des boîtes ; leur stockage compact dans les collections
reste à traiter. L'objectif d'absence presque totale d'allocations hors des
collections n'est donc pas intégralement satisfait.

### Arènes des valeurs récursives

La syntaxe est conservée. Lorsqu'une preuve fermée de possession est disponible,
les nœuds récursifs sont groupés dans des chunks de 64 Kio appartenant à la racine.
La construction utilise un bump, la destruction ordinaire libère les chunks sans
parcourir les arêtes, et une copie indépendante possède sa propre région.
Un builder possède un wrapper qui ouvre et ferme la région ; les appels récursifs
utilisent directement son corps, sans répéter ce protocole à chaque nœud.

La preuve actuelle est conservatrice : un seul fichier, pas de génériques ni de
code asynchrone, champs scalaires et liens vers le même type racine, pas de fuite
des racines dans des conteneurs, captures ou écritures globales. Les unions
récursives peuvent contenir des `Character`, et leurs builders peuvent lever une
erreur. Un caractère possédant un long buffer ajoute un cleanup à la région ;
les caractères statiques et inline n'en ajoutent pas. Les arbres partiellement
construits sont libérés sur erreur. Les autres formes gardent le chemin général
de possession : l'optimisation ne couvre donc pas encore tous les arbres Fa.

### Collections de caractères

Pour `characters()` et `split("")` d'un texte SIMPLE, le runtime conserve un
instantané compact d'un octet par caractère, plutôt qu'un tableau de valeurs de
24 octets. `get`, la longueur, la copie et la destruction utilisent cette forme.
Les opérations demandant des slots ordinaires matérialisent le tableau ; les
mutations conservent ainsi leur sémantique. Le texte source peut être réaffecté
sans invalider la collection. Les graphèmes Unicode gardent le chemin général.
Cette représentation est désactivée pour les programmes multi-fichiers, avec
code étranger ou boucles parallèles, tant que leur matérialisation partagée n'est
pas couverte par une preuve suffisante.

### Construction et lecture de chaînes

- Le chemin courant de `+=` met à jour l'en-tête local sans donner son adresse au
  runtime. Seul le chemin lent expose une copie de l'en-tête, ce qui permet de
  garder le pointeur, la longueur et la capacité en SSA entre les tours de boucle.
- Un template composé de chaînes locales possédées distinctes et de littéraux
  s'ajoute directement à sa destination, sans détour par le builder global.
  Les alias `use`, les autres expressions et la destination elle-même conservent
  le chemin général qui préserve l'instantané du membre droit.
- La recherche d'un `Character` dans un alphabet ASCII constant consécutif
  devient une soustraction et un test de plage. Les graphèmes multioctets ne
  correspondent pas ; les recherches de sous-chaînes et de chaîne vide gardent
  leur sémantique générale.
- L'itération `codePoints()` dispose d'un chemin de lecture d'octets sous le
  drapeau SIMPLE invariant. Elle n'a plus à tester le premier octet UTF-8 sur ce
  chemin. Les caractères issus d'un texte SIMPLE n'ont pas à être retestés pour
  `\r`, puisque le drapeau exclut déjà cet octet.
- L'égalité des chaînes compare des mots de 8, 4 et 2 octets puis le dernier
  octet, sans lecture au-delà de la fin de l'une des chaînes.
- Les templates de chaînes et d'entiers évaluent les expressions dans l'ordre,
  calculent la taille totale, puis écrivent directement dans le résultat. Un
  entier seul utilise directement une petite chaîne, sans builder partagé.
  `Array<String>.join` calcule également la taille finale avant la copie.
- Le padding d'un texte SIMPLE par un caractère ASCII sans `\r` utilise la
  longueur en octets, puis un remplissage direct. Les autres cas continuent à
  compter les graphèmes et à cycler les caractères du filler.
- Les petites boucles synchrones de caractères peuvent choisir le parcours
  SIMPLE une seule fois avant la boucle. Le corps reçoit alors un caractère
  d'un octet dont le drapeau est connu. La duplication est bornée et évitée
  dans les boucles imbriquées et le code asynchrone.

Le drapeau SIMPLE ne signifie pas « UTF-8 quelconque » : il garantit ASCII sans
`\r`. Les chemins Unicode restent présents. Les nouveaux tests couvrent accents,
graphèmes combinés, drapeau emoji, auto-concaténation et limites de buffers.

### La table décimale `000102…9899`

Cette constante de 200 octets énumère **toutes les paires décimales de 00 à 99**.
La conversion d'un entier écrit deux chiffres à la fois : elle indexe la table
par le reste modulo 100, puis divise l'entier par 100. La même routine sert à
l'impression, à `toString()` et aux templates, pour toute valeur signée ou non
signée de 64 bits. Les entiers plus larges conservent leur conversion générale.

La table ne contient aucune entrée ou sortie d'un benchmark. Aucune optimisation
du compilateur ne teste le nom d'un benchmark, sa taille ou son résultat attendu.
Les règles sont fondées sur les types, la forme du code et des preuves locales
de possession. Les tests comparent la conversion à `snprintf`, indépendamment
des benchmarks : bornes signées/non signées, voisinage des puissances de dix,
10 000 valeurs pseudo-aléatoires. Sur native, les résultats décimaux 64 bits
tiennent tous dans les 23 octets inline et n'allouent pas.

### Spécialisation des petits chemins et du tri

Les petites fonctions feuilles sans appel utilisateur sont inlinées afin que
les constantes et les faits du site d'appel simplifient leurs chemins génériques.
Le tri générique du runtime est également inliné : la taille des éléments et le
comparateur deviennent connus, ce qui supprime des copies génériques et des
appels indirects. L'algorithme reste le tri fusion stable existant.

Le runtime est conservé sous forme d'IR produit avec `clang -Os`. Pour une
compilation visant la vitesse, les attributs `optsize` et `minsize` de cet IR sont
retirés avant le pipeline O3 ; les attributs de chemins froids sont conservés.
La compilation visant la taille conserve les attributs de taille.

### Appels récursifs et divisions invariantes

Une petite base de fonction récursive pure et scalaire peut être évaluée au
site de l'appel récursif, après l'évaluation unique de ses arguments. Cette règle
s'applique à toute fonction répondant à ses conditions, sans nom prédéfini.
Pour `fib`, les deux fonctions LLVM optimisées ont désormais exactement les mêmes
nombres d'opérations : 3 `icmp`, 5 `br`, 3 `phi`, 3 `add`, 2 `call`, 1 `ret`.
Leur séquence d'instructions x86-64 est également identique après normalisation
des adresses. Cela concerne le noyau récursif, pas les exécutables complets ni
leur disposition dans les caches. Le temps de processus peut encore différer.

Certaines divisions 64 bits par une variable immuable hors de la boucle peuvent
préparer un réciproque entier, puis utiliser une multiplication haute et une
correction exacte. Le cas signé général conserve son calcul de division plancher
et ses gardes ; le raccourci positif ne change ni les négatifs ni la division par
zéro. Les tests couvrent notamment `UInt64.max`, `Integer.min`, les diviseurs de
1 à 256 et les diviseurs négatifs. Cette transformation peut augmenter le nombre
d'instructions tout en évitant une division matérielle dans la boucle : elle
doit donc être jugée aussi sur ses temps, et pas seulement sur `Ir`.

## Mesures

Instructions en **millions**, aux tailles d’audit. Temps CPU médians en secondes,
aux tailles par défaut ; le ratio est **Fa / Goose**. Un ratio supérieur à 1
signifie que Fa est plus lent sur ce relevé. Les deux échelles ne sont pas
interchangeables, particulièrement pour les effets de cache.

| Benchmark | Ir Fa avant | Ir Fa après | Ir Goose | CPU Fa | CPU Goose | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| binary-trees | 306.8 | 156.0 | 128.2 | 0.173 | 0.127 | 1.36× |
| blur | 96.7 | 96.7 | 81.0 | 0.103 | 0.114 | 0.90× |
| calc | 459.4 | 356.0 | 109.6 | 0.664 | 0.247 | 2.69× |
| fannkuch-redux | 136.3 | 138.5 | 108.3 | 0.254 | 0.268 | 0.95× |
| fib | 302.3 | 236.9 | 237.0 | 1.121 | 0.983 | 1.14× |
| graph | 12.2 | 10.9 | 7.0 | 0.272 | 0.443 | 0.61× |
| interp | 20.2 | 17.2 | 17.6 | 0.121 | 0.040 | 3.05× |
| lru | 37.1 | 38.8 | 17.9 | 0.791 | 0.255 | 3.11× |
| mandelbrot | 441.1 | 440.6 | 441.5 | 1.305 | 1.324 | 0.99× |
| nbody | 873.2 | 861.2 | 871.8 | 1.152 | 1.344 | 0.86× |
| particles | 105.7 | 105.7 | 105.6 | 0.142 | 0.141 | 1.01× |
| push | 135.2 | 135.2 | 121.5 | 0.350 | 0.327 | 1.07× |
| records | 163.3 | 135.7 | 73.4 | 0.394 | 0.211 | 1.87× |
| respond | 517.5 | 346.7 | 189.7 | 0.621 | 0.312 | 1.99× |
| scene | 12.9 | 9.8 | 7.9 | 0.101 | 0.019 | 5.26× |
| sexp | 239.7 | 198.6 | 77.5 | 0.409 | 0.196 | 2.09× |
| sieve | 203.7 | 203.7 | 249.5 | 1.096 | 0.960 | 1.14× |
| sort-objects | 281.4 | 160.8 | 119.4 | 0.429 | 0.375 | 1.14× |
| spectral-norm | 530.7 | 530.7 | 608.3 | 1.002 | 1.068 | 0.94× |
| string-building | 527.1 | 296.1 | 148.7 | 0.535 | 0.167 | 3.21× |
| strlist | 148.1 | 153.6 | 81.1 | 0.446 | 0.218 | 2.05× |
| sum | 158.6 | 158.6 | 142.7 | 0.313 | 0.301 | 1.04× |
| tree | 98.5 | 72.6 | 56.8 | 0.281 | 0.397 | 0.71× |
| word-count | 245.7 | 246.3 | 146.5 | 0.762 | 0.412 | 1.85× |
| words | 47.9 | 44.2 | 22.4 | 0.145 | 0.091 | 1.59× |

Les valeurs exactes, tailles, échantillons et empreintes sont dans
[le relevé JSON](native-goose-performance-results.json). Il conserve aussi les
comptes statiques LLVM du module et de `main` pour les deux compilateurs, et
ceux des noyaux de `fib`. Les totaux statiques incluent des chemins froids
et des choix d’inlining différents : ils ne sont pas utilisés comme score de vitesse.

**La condition des 25 benchmarks n’est pas satisfaite.** Les régressions en
instructions de `lru`, `strlist` et `fannkuch-redux` restent visibles ; une
amélioration du nombre d’allocations ne les annule pas. Les écarts temporels
modestes de la VM doivent être distingués des écarts importants encore présents.

## Pourquoi la minimalité et la parité ne sont pas encore acquises

1. **Collections et unions.** Les chaînes courtes n'allouent plus, mais restent
   des valeurs de 24 octets ; les grosses unions non récursives restent boxées. Les
   accès génériques aux collections et certaines boucles matérialisent des
   représentations qui pourraient rester compactes. Les tableaux temporaires
   de code points peuvent rester virtuels pour un `let` uniquement indexé ou
   mesuré dont la source est stable ; les autres usages restent matérialisés.
   Cette fusion existait avant cette branche : ce rapport ne la compte pas
   parmi les nouveaux gains.
2. **Arbres possédés.** Les arènes réduisent les allocations et suppressions
   individuelles, mais la preuve actuelle exclut encore certaines formes.
   Goose peut aussi utiliser des liens relatifs de taille inférieure aux
   pointeurs natifs. `sexp`, avec ses chaînes et tableaux dans les nœuds,
   conserve le chemin général.
3. **Tables.** Fa garde un hash et un état de présence par entrée, une indexation
   indirecte et des rehashes depuis une petite capacité. Les ports Goose de `lru`
   et `words` préallouent et réutilisent davantage leurs résultats de recherche.
   Le hash, la recherche puis la suppression ou l'insertion peuvent encore être
   répétés chez Fa.
4. **Indices non prouvés.** Les champs `previous` et `next` de `lru` sont des
   `Integer`, pas des références dont la plage est démontrée. L'analyseur signale
   donc `IndexError` et le programme écrit `try`. Ces checks doivent rester tant
   qu'une preuve n'est pas transmise au backend. L'absence de référence pendante
   ne constitue pas une preuve d'indice valide.
5. **Taille constante côté Goose.** Pour quinze ports, Goose reçoit `N` comme
   constante de compilation ; Fa lit un argument. Les divisions par une taille
   connue chez Goose sont particulièrement favorisées dans `lru` et `words`.
   Ce protocole existant est conservé et cette différence n'est pas masquée.
6. **Instructions, caches et cycles.** Une baisse du nombre d'instructions ne
   garantit pas le même gain de temps : les allocations, dispositions mémoire,
   défauts de cache et divisions peuvent dominer. Les grands jeux de données
   restent nécessaires, notamment pour `interp` et `scene`.

La suite logique est d'étendre les représentations compactes et les régions aux
formes encore exclues, puis les preuves de plages et la réutilisation de
recherches de tables. Chaque changement doit être accepté
sur ses résultats corrects, ses instructions et ses temps sur les 25 programmes.
La condition finale reste une comparaison par benchmark, pas une moyenne globale.

## Validation

Les vérifications suivantes passent avec LLVM 18 activé :

- `cargo fmt --all` et vérification du format ;
- `cargo test --workspace` ;
- `cargo clippy --workspace --all-targets` ;
- `cargo build --release -p fa` ;
- les 19 tests ciblés d'IR, dont les vérifications de passage par référence,
  de longueur constante et de stockage statique des membres sans données ;
- les programmes sur native, native sans libc et wasm, y compris leurs
  vérifications de fuites ;
- le runtime C sous ASan et UBSan, exécuté aussi directement ;
- les conversions décimales contre `snprintf`, les limites et les valeurs
  pseudo-aléatoires, sans allocation native ;
- le programme des unions inline à `-O0` avec vérification des fuites,
  et le programme de passage de constantes `static` par référence ;
- la reconstruction des 25 programmes : mêmes binaires que ceux mesurés,
  sorties à nouveau comparées à l'oracle conservé.

Les cas de correction couvrent aussi les copies indépendantes, les mutations,
les erreurs pendant la construction d'une arène, les caractères Unicode, les
vues, les lambdas et les appels indirects. Les programmes de benchmark n'ont
pas été modifiés pour les faire passer.

Une exécution initiale, sous forte charge de profilage, avait échoué sur
l'assertion temporelle wasm d'`async-starvation` (au plus deux tours occupés
pendant 20 ms). Trente répétitions isolées puis les suites complètes suivantes
ont passé sans modification de ce test. Les temps retenus ont été mesurés
sans travail de compilation ni de profilage simultané.

Le test CLI de rechargement pouvait observer la dernière ligne de stdout avant
que le watcher n'observe la fin du processus. Son assertion accepte désormais
les deux messages déjà prévus par `develop` : redémarrage du processus vivant
ou nouvelle exécution du processus terminé. Les assertions sur la nouvelle
sortie et le code de sortie restent présentes. La suite complète passe avec
ce correctif de synchronisation du test.

## Deuxième itération : stockage stable et preuves de possession

Cette itération part de `b5cfecd`. Les résultats et empreintes propres à cette
itération sont conservés dans
[le second relevé](native-goose-optimization-results.json). Le tableau historique
ci-dessus reste celui de la première itération.

### Changements

- **Stabilité des `Map`.** L'ancien backend pouvait déplacer une entrée lors
  d'une insertion, contrairement à la garantie documentée. Les `Map` utilisent
  maintenant des blocs fixes de 32 entrées. Seul le répertoire de pointeurs peut
  bouger. Les slots supprimés sont réutilisés par une liste libre stockée dans
  leur ancien mot de hash ; ni allocation ni métadonnée supplémentaire par slot
  libre. L'ordre des slots est utilisé à l'itération ; le langage ne promet pas
  d'ordre pour une `Map`. Les `Set` ordinaires gardent leur stockage déplaçable.
- **Index compact avec repli.** Jusqu'à `2^31` slots d'index, la charge maximale
  de moitié prouve que les numéros d'entrée tiennent dans un entier signé de
  32 bits. Au-delà, l'index utilise 64 bits. Aucune troncature ni limite de taille
  du langage n'est ajoutée. Les octets de l'index sont divisés par deux dans le
  cas courant ; il ne s'agit pas de diviser toute la mémoire de la table par deux.
- **Réutilisation des sondages.** L'insertion conserve le premier slot libre du
  sondage qui a établi l'absence, tout en recherchant une éventuelle clé après
  une tombstone. Un rehash ou une compaction force un nouveau sondage. Les
  recherches en lecture seule évitent ce suivi supplémentaire. Le mot de présence
  d'une entrée vivante contient son slot d'index, actualisé lors d'un rehash : la
  suppression ne refait plus la recherche. Les sondages ne sont jamais conservés
  à travers un appel utilisateur ou une mutation arbitraire.
- **Churn borné.** Les tombstones sont comptées indépendamment des slots
  d'entrée. À trois quarts d'occupation de l'index, une reconstruction conserve
  des emplacements vides et garantit la terminaison des recherches. Dix mille
  insertions/suppressions avec une seule entrée permanente n'augmentent ni le
  nombre de slots utilisés ni la capacité de la table de test.
- **Arènes et payloads possédés.** Les unions récursives éligibles acceptent
  maintenant des `String`, des tableaux non récursifs et leurs agrégats finis,
  sans destructeur utilisateur. Les ressources possédées sont libérées par les
  cleanups de la région ; les arêtes récursives ne sont pas parcourues par ces
  callbacks. Les copies sont indépendantes et une construction interrompue
  libère ses payloads. Les mutations par appel et par alias excluent l'arène,
  de même que les appels étrangers. Les conteneurs de racines, tableaux d'enfants
  récursifs, captures, génériques et programmes asynchrones restent exclus.

Les blocs fixes évitent le calcul logarithmique d'adresse des segments
géométriques. Ils ont été retenus après profilage du premier essai, plus coûteux
en instructions. Les petits tableaux, chaînes et objets contenus dans les
payloads ne sont pas eux-mêmes placés inline dans les nœuds par ce changement.
`scene` et `sexp` restent donc des travaux ouverts.
Un premier bloc réserve 32 slots : une toute petite `Map` peut consommer plus de
mémoire qu'auparavant. La réduction du pic des grands cas ne démontre donc pas
une compacité minimale pour toutes les tailles.

### N fourni à l'exécution

Le pilote possède désormais `--goose-runtime-size`. Il ne modifie que les copies
générées des sources Goose : `BENCH_N` et ses constantes globales entières
dérivées sont initialisés depuis `argv`, au début de `main`. Ces globals sont
mutables dans cette variante ; cette différence avec des paramètres immuables
doit être conservée à l'esprit lors de l'interprétation des résultats.
Les annotations `checkfree` restent inchangées. Le mode par défaut conserve le
protocole avec constantes, et chaque résultat indique `goose_size`.

Les 24 binaires Goose acceptés dans ce mode ont été comparés à Rust aux deux
échelles, sans recompilation entre les tailles. `blur` échoue à compiler : les
relations entre le `W` dynamique et les tailles des tableaux ne sont plus
prouvées pour `checkfree`. Cet échec est conservé dans le relevé, sans retrait de
l'annotation ni remplacement par une variante moins vérifiée.

```sh
python3 -m unittest discover -s benchmarks/audit -p 'test_*.py'
python3 benchmarks/audit/compare.py --directory benchmarks/audit/build/runtime-n \
  --goose-runtime-size --default-sizes --cpu 1 --runs 7
```

### Optimisations encore possibles et preuve nécessaire

| Priorité | Optimisation générique | Condition de sécurité / validation |
| --- | --- | --- |
| Haute | Payloads d'unions de taille variable dans un buffer de collection, avec index d'offsets | Conserver l'accès aléatoire ; aligner les payloads ; promouvoir les offsets ou matérialiser avant débordement ; traiter mutations, copies et références invalidées. Mesurer `records` à grande taille, pas seulement les allocations. |
| Haute | Arènes traversant `Array<Root>` et collections de racines | Identifier le propriétaire de chaque région ; libérer les buffers sans détruire deux fois les enfants ; couvrir copies de sous-arbres, extractions, erreurs et mutations. Débloque notamment `sexp`. |
| Haute | Nœuds à petits tableaux intégrés | Prouver une borne de longueur sur tous les chemins de construction et d'écriture, puis conserver un repli pour les autres usages. Le nom `maxChildren` ou une taille observée ne constitue pas une preuve. Cible : `scene`. |
| Haute | Groupes de contrôle et empreintes de hash pour l'index | Filtrer les collisions avant de charger les entrées ; garder leurs adresses stables ; gérer les groupes à la frontière sans lecture hors allocation. Mesurer les chaînes et les entiers séparément. |
| Haute | Réutiliser `includes/get` puis `[]`, `delete` ou `getOrInsert` | Attacher le sondage à une version de la collection et une identité de clé ; invalider la preuve sur mutation, effet inconnu et alias. Le présent changement couvre seulement le sondage interne à une insertion/suppression. |
| Haute | Liens relatifs ou numéros de nœuds sur 32 bits | Prouver la plage, ou promouvoir avec mise à jour des liens avant dépassement ; jamais tronquer un pointeur. Ne pas imposer une limite cachée de 4 Gio à un programme valide. |
| Moyenne | Hash et présence plus petits dans les entrées | Garder assez d'information pour reconstruire l'index et distinguer toutes les positions ; égalité structurelle finale obligatoire sur collision. Séparer bitmap d'occupation et payload pour les scans clairsemés. |
| Moyenne | Comparaison par hash seul pour certaines clés scalaires | Démontrer que le hash est injectif pour la représentation exacte de la clé, comme une permutation sur 64 bits pour un entier de 64 bits au plus. Lier cette preuve à la fonction de hash utilisée ; les chaînes et agrégats gardent leur comparaison complète. |
| Moyenne | Vues de clés et de sous-chaînes sans copie | La source doit rester vivante et ses octets inchangés pendant tout l'emprunt ; promotion en copie possédée si la preuve cesse. Couvrir réaffectation du texte et clés Unicode. Cible : `words`. |
| Moyenne | Préallocation déduite des boucles et cardinalités | Déduire une borne correcte de production, en tenant compte des branches et appels. Éviter une réserve excessive pour une `Map` à très peu de clés distinctes. |
| Moyenne | Petites `Map` avec premier stockage intégré | Garder les premières entrées à une adresse stable dans l'allocation de l'en-tête, puis allouer les blocs suivants ; mesurer le coût de la branche d'adressage et les programmes créant beaucoup de petites tables. |
| Moyenne | Analyse relationnelle des indices et longueurs | Transmettre les inégalités et identités de collection au LLVM ; les invalider après raccourcissement ou effet inconnu. Tester indices négatifs, références imbriquées et erreurs réellement atteignables. |
| Moyenne | Diviseurs invariants partagés entre fonctions | Remonter le calcul du réciproque au propriétaire de la boucle ; conserver les cas zéro, minimum signé et division plancher. Évaluer avec `N` dynamique des deux côtés. |
| Moyenne | Fusion des scans et temporaires de chaînes | Fusionner `split/filter/count`, `codePoints` et templates lorsque les valeurs intermédiaires n'échappent pas ; conserver l'ordre d'évaluation, les instantanés et les limites de graphèmes. |
| Moyenne | Construction directement dans la destination et copies évitées | Dernier usage prouvé, aucun alias survivant, nettoyage des constructions partielles. Appliquer aux unions, arguments, retours et éléments de collection. |
| Basse | Spécialisations guidées par les effets et le profil | Réserver les chemins ASCII, petits objets et appels directs aux preuves disponibles ; borner la duplication du code et mesurer caches, taille binaire et temps en plus des instructions. |

La parité avec Goose sur les 25 programmes reste un objectif ouvert. Une baisse
de mémoire, un correctif de stabilité et une baisse de temps CPU sont des
résultats distincts ; aucun ne justifie de supprimer un check non prouvé.

### Validation de la deuxième itération

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets` et
  `cargo test --workspace` passent avec LLVM 18 activé, doctests compris.
- Le runtime C passe ASan et UBSan : collisions forcées, recherche et insertion
  derrière une tombstone, rehashes, copies, effacement, réutilisation des slots,
  largeur de l'index et stabilité des pointeurs.
- Les nouveaux programmes `stable-map.fa` et `passive-tree-arenas.fa` passent
  sur native, native sans libc et wasm, avec détection des fuites. Ils passent
  également à `-O0`. Le cas de mutation d'un payload via un alias passe à `-O0`
  sur les trois cibles, en conservant le chemin général de possession.
- Les tests d'IR vérifient l'usage des arènes pour les payloads finis et leur
  exclusion lorsque ces payloads sont modifiés par un helper via un alias.
- Les trois tests Python du nouveau protocole passent ; les sources originales
  des benchmarks ne sont pas modifiées.
- Les 25 exécutables Fa avant/après ont été reconstruits et leurs sorties
  comparées à Rust. Vingt et un sont identiques octet par octet avant/après ;
  seuls `calc`, `lru`, `word-count` et `words` ont un SHA-256 différent.
  La section machine `.text` de `calc` est également identique.

Les mesures finales sont prises après la fin des compilations, tests et profils.

### Résultats de la deuxième itération

Le gain le plus net est la mémoire et la conformité des références de `Map`.
Les temps sont des médianes de sept échantillons alternés sur le CPU 1, aux
tailles par défaut. Les instructions sont les totaux dynamiques Callgrind aux
tailles d'audit ; la mémoire est le pic RSS du processus aux tailles par défaut,
relevé séparément avec `/usr/bin/time`.

| Cas | CPU avant → après (s) | Instructions avant → après (millions) | Pic RSS avant → après (Mio) |
| --- | ---: | ---: | ---: |
| lru | 0,726 → 0,706 | 38,82 → 42,64 | 59,50 → 18,88 |
| word-count | 0,893 → 0,762 | 246,33 → 250,87 | 9,25 → 5,88 |
| words | 0,137 → 0,118 | 44,17 → 46,55 | 46,50 → 46,50 |

Le pic RSS baisse d'environ **68 % sur `lru` et 36 % sur `word-count`**. Le pic
global de `words` ne baisse pas, malgré l'index plus petit. Les instructions
augmentent respectivement de 9,9 %, 1,8 % et 5,4 % sur ces trois programmes : le
coût des adresses stables et du nouvel index reste visible et demande encore
du travail. Les anciens binaires de référence ne respectaient pas la garantie
de stabilité lors d'une croissance ; leur stockage plat ne peut pas être
réintroduit sans preuve que les références concernées ne survivent pas.

Les baisses des médianes CPU observées sont de 2,7 %, 14,7 % et 13,7 %, mais les
échantillons se recouvrent largement. Par exemple, `sum`, dont le binaire est
strictement identique, varie de 0,254 à 0,287 s entre les deux séries. Ces mesures
sur VM partagée ne suffisent donc pas à garantir un gain de vitesse de ces
pourcentages. Le relevé conserve tous les échantillons, les commandes et les
empreintes pour éviter de confondre baisse de mémoire, baisse d'instructions et
accélération.

Pour `lru`, le Goose historique prend 0,227 s et la variante `N` dynamique
0,259 s ; Fa prend 0,706 s. Le fait de fournir `N` à l'exécution réduit une
partie de l'écart, sans l'annuler. Sur les 25 ports historiques, 15 reçoivent une
constante et 10 lisaient déjà `argv`. Le second relevé indique ce mode par
programme et conserve le refus de compilation de `blur` en mode dynamique.
**La condition de parité sur chacun des 25 benchmarks reste non satisfaite.**

## Troisième itération : alignement, chaînes et recherches

Cette étape conserve des changements locaux au backend et au runtime, sans modifier les programmes de comparaison ni la syntaxe Fa.

| Changement | Condition de validité |
| --- | --- |
| Arènes : granularité de 8 octets, avec padding supplémentaire pour les types plus alignés. Un nœud de 24 octets ne consomme plus 32 octets. | Alignement calculé par LLVM pour la cible ; les types de 128 bits et les enregistrements de nettoyage gardent leur alignement. |
| Lecture d’une collection ASCII compacte : valeur inline d’un octet. Égalité vide ou d’un octet visible dans l’appelant. | Longueur vérifiée avant lecture ; chemin général conservé pour les autres chaînes et les collections matérialisées. |
| Largeur de l’index choisie avant la boucle de sondage. | Le masque de l’index reste inchangé pendant une égalité structurelle ; chemin 64 bits conservé. |
| Suppression du comparateur de clé après égalité du hash complet pour les entiers d’au plus 64 bits. | Extension injective puis mélange bijectif : xor-shifts et multiplications impaires modulo 2^64. Les entiers de 128 bits et les autres clés gardent leur comparateur. |
| Constructeur d’`arguments` : borne de son compte rendue visible et en-tête terminé après la copie des valeurs. | Le compte natif vient de `argc: i32`, celui de wasm tient sur 32 bits. La capacité ordinaire reste prouvable malgré les appels d’allocation et la copie. Cette borne porte sur le nombre d’arguments, jamais sur la valeur de `N` parsée. |

Les contrôles de bornes, les règles Unicode, les copies de valeurs et la stabilité des références de `Map` sont conservés. Les liens et les entiers des nœuds restent à leur largeur actuelle.

### Validation

Le workspace complet passe avec LLVM 18 ; après le dernier ajustement du constructeur C, tous les tests du backend LLVM repassent, dont les programmes sur les trois cibles et le runtime sous ASan/UBSan. Formatage et Clippy passent. Six programmes ciblés passent aussi à `-O0`, avec vérifications d’overflow et détection des fuites : entiers de 64/128 bits dans les tables, caractères et Unicode, arbres à payloads de 128 bits, payloads possédés, références stables et arguments.

Les tests forcent des collisions de hash entre clés distinctes de 128 bits, des collisions de buckets avec comparateur omis, les deux largeurs d’index, les tombstones, les changements de page et des alignements mélangés. Le constructeur d’arguments est testé avec des comptes natifs négatifs, nuls et positifs.

### Instructions et mémoire

| Cas | Instructions avant → après (millions) | Pic RSS avant → après (Mio) |
| --- | ---: | ---: |
| calc | 355,992 → 341,054 | 1,38 → 1,38 |
| interp | 17,147 → 17,147 | 12,88 → 11,00 |
| tree | 72,639 → 72,636 | 130,25 → 98,00 |
| lru | 42,640 → 42,533 | 18,88 → 18,88 |
| word-count | 250,866 → 243,945 | 5,88 → 5,88 |
| words | 46,549 → 45,649 | 46,50 → 46,50 |
| scene | 9,746 → 9,746 | 7,38 → 7,38 |

La baisse de RSS est d’environ 15 % sur `interp` et 25 % sur `tree`. Les instructions baissent d’environ 4,2 % sur `calc`, 2,8 % sur `word-count`, 1,9 % sur `words` et seulement 0,25 % sur `lru`. Les baisses mémoire ne sont pas des baisses d’instructions : `interp` en exécute autant.

### Temps Fa et Goose avec N dynamique

Les valeurs ci-dessous sont les médianes CPU de sept échantillons alternés sur le CPU 1, après les compilations, les tests et les profils. Le protocole conserve `-O3` et les overflow checks désactivés, comme lors de l’itération précédente ; les changements sont également validés avec ces checks activés.

| Cas | Fa avant (s) | Fa après (s) | Goose N dynamique (s) | Fa après / Goose |
| --- | ---: | ---: | ---: | ---: |
| binary-trees | 0,1690 | 0,1939 | 0,1445 | 1,34× |
| blur | 0,1014 | 0,0887 | refus checkfree | — |
| calc | 0,6674 | 0,6125 | 0,2471 | 2,48× |
| fannkuch-redux | 0,2673 | 0,3031 | 0,2481 | 1,22× |
| fib | 1,0925 | 1,0793 | 1,0626 | 1,02× |
| graph | 0,2462 | 0,2539 | 0,4285 | 0,59× |
| interp | 0,1341 | 0,0922 | 0,0433 | 2,13× |
| lru | 0,7966 | 0,9011 | 0,2892 | 3,12× |
| mandelbrot | 1,3863 | 1,2598 | 1,2207 | 1,03× |
| nbody | 1,2200 | 1,1495 | 1,2546 | 0,92× |
| particles | 0,1295 | 0,1379 | 0,1219 | 1,13× |
| push | 0,3565 | 0,3597 | 0,3543 | 1,02× |
| records | 0,4472 | 0,4280 | 0,2173 | 1,97× |
| respond | 0,7136 | 0,6747 | 0,2669 | 2,53× |
| scene | 0,1165 | 0,1169 | 0,0245 | 4,78× |
| sexp | 0,4233 | 0,4100 | 0,1954 | 2,10× |
| sieve | 1,0736 | 1,1492 | 1,0576 | 1,09× |
| sort-objects | 0,4355 | 0,4612 | 0,3988 | 1,16× |
| spectral-norm | 0,9568 | 1,0564 | 1,0519 | 1,00× |
| string-building | 0,5251 | 0,5459 | 0,1551 | 3,52× |
| strlist | 0,3137 | 0,3148 | 0,1661 | 1,90× |
| sum | 0,2723 | 0,2674 | 0,2712 | 0,99× |
| tree | 0,2461 | 0,2030 | 0,3758 | 0,54× |
| word-count | 0,6741 | 0,6450 | 0,3690 | 1,75× |
| words | 0,1245 | 0,1196 | 0,0744 | 1,61× |

À profondeur 23, le même exécutable `interp` passe de 0,3765 à 0,3053 s ; Goose dynamique prend 0,1352 s. Les sorties sont égales aux deux tailles, sans recompiler.

Les temps restent dispersés sur cette VM partagée. Certaines médianes régressent : la baisse d’instructions ne suffit donc pas à annoncer une accélération CPU sur toutes les tables ou tous les programmes. Les résultats bruts conservent tous les échantillons, les empreintes, une répétition ciblée et un contrôle exécutant le même binaire sous deux étiquettes. Les chiffres CPU sont des observations, pas une garantie de gain reproductible sur une machine dédiée.

Le contrôle avec le même binaire `sum` donne deux médianes de 0,2444 et 0,2520 s, soit 3,1 % d’écart sans changement de code. Sur `lru`, la série principale donne 0,7966 → 0,9011 s, tandis qu’une répétition alternée de neuf échantillons du même couple d’exécutables donne 0,7094 → 0,6741 s. Aucun gain CPU stable n’est donc établi sur `lru`.

La parité sur les 25 cas reste ouverte. Le stockage de `scene` demande encore une preuve qui sépare les mutations de données de celles de la topologie, puis des collections de liens compactes. Pour les tables, le filtrage par empreintes avant lecture des entrées et la réutilisation des recherches avec preuve sur les clés et les mutations restent à faire.

Les [données de cette itération](native-goose-round3-results.json) rassemblent tous les échantillons finaux et les contrôles de mesure. Les sources originales des benchmarks sont inchangées ; le refus `checkfree` de `blur` à taille dynamique est conservé.

## Quatrième itération : stockage récursif et `scene`

Audit du 4 novembre 2026, dans l'arbre de travail. Les changements sont choisis
selon les types, les usages prouvés et la plateforme ; aucun ne reconnaît un
benchmark, une taille d'entrée ou un résultat attendu.

### Audit des boîtes redondantes

L'arène existante ne réglait pas le choix de représentation des éléments de
collection : une arête `Rec` ajoutait une boîte même lorsque le slot fournissait
déjà le stockage de l'objet. La règle est maintenant partagée par les tableaux,
les entrées de tables, les tableaux croissants, les pools et les columnars.

| Cas | Représentation et garantie |
| --- | --- |
| Alias récursif d'une collection | Réutilise la représentation du conteneur, sans seconde boîte `Rec`. Les variantes `Growing` suivent les mêmes chemins. |
| Objet ou tuple dont les cycles passent par des collections | Une analyse du graphe complet démontre que le layout par valeur est fini, y compris avec récursion mutuelle. Les types génériques non résolus restent conservateurs. |
| Élément `Node` contenant aussi `next: Node?` | Le slot de collection contient l'objet ; le champ récursif direct conserve son indirection. |
| Callback Array/Map/Set/Pool sur un élément direct | Emprunte l'adresse du slot d'origine lorsque sa signature attend un `Rec` de cet objet, sans allocation d'adaptation. |
| `Rec` vers une union déjà représentée | Réutilise la représentation de l'union, inline ou à pointeur. Cela ne supprime pas la boîte propre à une union à pointeur. |

La construction, les copies profondes, les extractions et les réinsertions
emploient le layout réellement stocké. Les résultats optionnels de `get`, `find`,
`pop` et `take` ne doivent ni détruire le stockage emprunté ni perdre un payload
possédé. Les références des `Map` continuent à désigner leurs emplacements stables
pendant les insertions.

L'audit ne conclut pas à l'absence de toute allocation évitable. `Array<Node?>`
et les `Rec` imbriqués dans certains tuples ou agrégats conservent leur
représentation actuelle. Une union contenant une collection peut encore être
exclue du critère d'inlining, même si son layout serait fini. Un callback sur une
ligne de columnar répartie en colonnes peut aussi demander une adaptation en
objet contigu. Ces cas demandent un travail de représentation distinct. Pour
la récursion directe, l'arène reste sélectionnée sous ses preuves existantes ;
les autres cas gardent le chemin général de possession.

### Changements complémentaires mesurés

- Le parcours d'un tableau charge son pointeur et sa longueur avant la boucle.
  La règle d'itération interdit déjà les modifications de structure, y compris
  par alias et appels ; les modifications des éléments restent visibles.
- Une boucle comptée peut réserver les éléments qu'elle ajoute à un champ
  tableau initialement vide. Les checks de capacité restent dans chaque push :
  un appel par alias peut ajouter des éléments supplémentaires. Un champ non
  vide conserve son allocation jusqu'au push, afin de préserver ses références.
- Un retour direct d'un local passif peut déplacer sa valeur à chaque sortie.
  La preuve exclut notamment les captures, les alias, les ressources à
  destruction personnalisée et les fonctions asynchrones.
- La conversion Float vers Integer a une seule garde chaude ordonnée sur
  `[-2^63, 2^63[`. NaN, infinis et dépassements gardent les erreurs existantes ;
  leur classification détaillée s'exécute dans le chemin froid.
- L'allocateur réutilise des spans natifs de 2 Mio, comptabilisés entièrement
  dans ses pages libres ; wasm32 conserve ses refills de 1 Mio. Sur Linux,
  les spans complets sont alignés sur 2 Mio et reçoivent `MADV_HUGEPAGE`.
  Un échec d'allocation retente l'alignement ordinaire et le refus du conseil
  est ignoré. Ce choix augmente parfois le RSS des petits programmes.

Supprimer seulement la boîte des enfants ne suffisait pas à accélérer `scene` :
la capacité initiale de quatre objets de 112 octets gaspillait l'espace pour les
petits tableaux. La réservation exacte améliore la densité. Les grandes pages
réduisent ensuite le coût de traduction des adresses sur cette machine, où les
pages transparentes sont configurées en mode `madvise`.

Les fonctions de formatage et d'égalité des composites sont mises en cache avant
la génération de leur contenu, pour produire des appels récursifs finis. Les
payloads optionnels ne sont comparés que lorsque les deux côtés sont présents.
Les vues temporaires et les conversions d'arguments sont nettoyées dans la
branche qui les crée. L'égalité ordinaire conserve `NaN != NaN`, séparément de
la comparaison de clés SameValueZero, propagée récursivement dans leurs champs,
éléments et valeurs. Les caches des deux modes restent distincts.

### Mesures finales de la quatrième itération

Médianes CPU en secondes, sur 45 passages alternés par taille, CPU 1. Le même
exécutable reçoit les trois profondeurs à l’exécution, pour les deux langages.
Les sources des benchmarks sont inchangées.

| Profondeur | Fa avant | Fa après | Goose | Fa / Goose |
| --- | ---: | ---: | ---: | ---: |
| 9 | 0,0028 | 0,0025 | 0,0027 | 0,91× |
| 12 | 0,1006 | 0,0180 | 0,0181 | 0,99× |
| 13 | 0,2573 | 0,0477 | 0,0524 | 0,91× |

Au cas standard, Fa est accéléré de **5,58×** par rapport
à son ancien exécutable remesuré dans la même série. La parité avec Goose est
observée ; leur très petit écart ne suffit pas à affirmer que Fa est plus rapide.
La variation sur une VM partagée reste visible, même en temps CPU.

Le pic RSS de `scene` passe de 7,38 à 9,25 Mio ; Goose prend 6,50 Mio.
Les grandes pages constituent un compromis de vitesse contre mémoire résidente,
avec un repli correct lorsque le système les refuse.

### Contrôle des autres programmes

Les 25 Fa donnent la sortie attendue de Rust. Le tableau suivant conserve la
série exploratoire de cinq passages alternés ; pour `scene`, préférer la série
de 45 passages ci-dessus. Goose reste indisponible sur `blur` avec `N` dynamique,
car sa preuve `checkfree` échoue ; l’annotation reste présente.

| Cas | CPU Fa avant | CPU Fa après | CPU Goose | Fa / Goose |
| --- | ---: | ---: | ---: | ---: |
| binary-trees | 0,1465 | 0,1493 | 0,1068 | 1,40× |
| blur | 0,0860 | 0,0826 | — | — |
| calc | 0,5256 | 0,5051 | 0,2054 | 2,46× |
| fannkuch-redux | 0,2365 | 0,2557 | 0,2247 | 1,14× |
| fib | 0,9971 | 0,8245 | 0,9208 | 0,90× |
| graph | 0,2117 | 0,1459 | 0,3850 | 0,38× |
| interp | 0,0819 | 0,0327 | 0,0398 | 0,82× |
| lru | 0,7532 | 0,6728 | 0,2707 | 2,49× |
| mandelbrot | 1,0954 | 1,0903 | 1,1072 | 0,98× |
| nbody | 0,9961 | 1,0090 | 1,0950 | 0,92× |
| particles | 0,1203 | 0,1167 | 0,1212 | 0,96× |
| push | 0,3203 | 0,3181 | 0,3291 | 0,97× |
| records | 0,4075 | 0,3854 | 0,2382 | 1,62× |
| respond | 0,5832 | 0,5942 | 0,2678 | 2,22× |
| scene | 0,0965 | 0,0168 | 0,0196 | 0,86× |
| sexp | 0,3740 | 0,3205 | 0,1775 | 1,81× |
| sieve | 0,9707 | 0,9904 | 0,9507 | 1,04× |
| sort-objects | 0,4319 | 0,4407 | 0,3834 | 1,15× |
| spectral-norm | 0,8914 | 0,9011 | 0,9572 | 0,94× |
| string-building | 0,4831 | 0,4776 | 0,1526 | 3,13× |
| strlist | 0,3087 | 0,3062 | 0,1601 | 1,91× |
| sum | 0,2719 | 0,2621 | 0,2709 | 0,97× |
| tree | 0,1941 | 0,1285 | 0,3341 | 0,38× |
| word-count | 0,5881 | 0,5682 | 0,3524 | 1,61× |
| words | 0,1039 | 0,1005 | 0,0689 | 1,46× |

Un contrôle exécutant le même binaire `sum` sous deux étiquettes donne 0,2859
et 0,2591 s. La hausse initiale de `fannkuch-redux` ne se reproduit pas
sur quinze nouveaux passages alternés : 0,2600 → 0,2600 s.
Ces contrôles interdisent de transformer toutes les petites différences de ce
tableau en gains ou régressions certains. La parité sur les 25 programmes reste
un objectif ouvert, notamment pour les unions et les tables.

### Validation finale

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets` et
  `cargo test --workspace` passent avec LLVM 18 activé, doctests compris.
- Les programmes du backend passent sur native, native sans libc et wasm ; le
  runtime C passe ASan/UBSan et les boucles d’allocation passent sous 128 Mio.
- 44 exécutions ciblées passent à `-O0` et `-O3`, avec overflow et détection
  des fuites. Les 4 exclusions concernent uniquement le formatage flottant
  non implémenté sans libc. Les huit fixtures natives passent aussi avec refus
  forcé des allocations alignées sur 2 Mio et de `madvise`.
- `scene` passe aussi aux trois profondeurs sur native et wasm, avec les
  checks d’overflow et de fuites activés.
- Les tests couvrent aussi les callbacks empruntant les slots, les copies
  indépendantes après destruction des originaux, les références stables et les
  deux modes de comparaison des composites contenant NaN.
- Après le dernier correctif d’égalité des clés, les 25 exécutables sont
  reconstruits et vérifiés : tous sont identiques octet par octet aux binaires
  chronométrés. Le relevé conserve leurs SHA-256.

Les [données complètes](native-goose-round4-results.json) contiennent les
échantillons, commandes, tailles, RSS, empreintes et contrôles de mesure.

## String-building : construction des chaînes

Travail du 4 au 14 novembre 2026, commits `8c376ef` et `86b3c1c`. Comme
précédemment, aucun changement ne reconnaît un benchmark, une taille ou un
résultat attendu, et les sources des benchmarks sont inchangées.

### Changements implémentés

- **Entiers en texte.** `toString` et les parties entières d'un template écrivent
  leurs chiffres dans un tampon de 24 octets sur la pile (`fa_decimal_i64`,
  `fa_decimal_u64`). La chaîne obtenue est une vue non possédante (genre
  `KIND_MASK`, comme les vues de caractères) : un consommateur immédiat (padding,
  interpolation, comparaison) la lit sans copie, et tout propriétaire (`let`,
  champ, élément, retour de fonction, valeur d'un `if`) la copie par `take`.
  Dans une coroutine, la suite de l'instruction peut suspendre (`await`) : la vue
  désignerait alors une pile abandonnée, et le texte reste une chaîne possédée
  temporaire. La conversion divise sur 32 bits dès que la valeur y tient.
- **Templates.** Les templates de chaînes et d'entiers évaluent toutes leurs
  parties, calculent la taille exacte avec contrôle de débordement
  (`fa_string_size`), allouent une fois (`fa_string_allocate`) et copient chaque
  partie. Le drapeau SIMPLE est le ET des drapeaux des parties ; seul un résultat
  non SIMPLE est rebalayé (`fa_string_finish`).
- **Réutilisation du buffer.** `s = "…{x}…"` sur un local `mutable` de type
  `String` écrit dans le buffer possédé existant lorsqu'il suffit
  (`fa_string_replace_buffer`), et `s = ""` conserve sa capacité. Ce chemin
  n'accepte comme parties textuelles que des locaux `let`/`mutable` distincts de
  la destination ; références `use`, champs, appels et lecture de la destination
  gardent le chemin qui alloue. Le slot de la destination est relu après
  l'évaluation des parties.
- **`parts.join(separator).length`** devient `fa_array_join_length` : si le
  séparateur et chaque partie non vide sont SIMPLE, la longueur en graphèmes est
  la somme des octets ; sinon la jointure est construite puis mesurée, car un
  graphème peut traverser la frontière de deux parties (`\r` + `\n`, marques
  combinantes, ZWJ). Le receveur puis le séparateur sont évalués dans l'ordre.
- **Runtime.** Copies courtes jusqu'à 32 octets sans appel, remplissage borné
  du padding, recherche de séparateur par blocs de 16 octets. La croissance du
  builder de texte copie par `memcpy` : avec les branches courtes de 32 octets,
  `reserve` n'était plus inlinée et `respond` l'appelait hors ligne à chaque
  ajout (+3,5 % d'instructions avant ce correctif).

### Corrections issues de la revue

- Une partie textuelle empruntée pouvait être modifiée par une partie suivante :
  dans `"{other}:{rewrite()}"`, si `rewrite` réécrivait `other` en place, le
  résultat contenait le nouveau texte. Le défaut existait déjà à `f75ce8a`
  (`fa_strings_join`). Une partie empruntée est désormais copiée lorsqu'une
  partie ultérieure n'est pas inerte (appel, `await`, affectation) ; noms,
  littéraux, accès, index et opérateurs ne déclenchent pas de copie, si bien que
  les templates de `string-building` n'en font aucune.
- Les chiffres d'une vue décimale lue avant un `await` de la même instruction
  étaient perdus à `-O0` après la reprise (`"{n}:{await f()}"` donnait `\0`
  au lieu de `5`) : d'où le chemin possédé dans les coroutines.
- Le chemin fusionné de `join(…).length` exige un appel de type `String` (pas un
  chaînage optionnel), et la réutilisation du buffer exige un slot `String`.

Restent hors de ce travail, constatés à `f75ce8a` : `maybe?.join("-").length`
fait paniquer le compilateur, et l'ordre d'évaluation de `a + f()` lorsque `f`
modifie `a` n'a pas été audité.

### Validation

- `cargo fmt --all`, `cargo clippy --workspace --all-targets` sans
  avertissement, et `cargo test --workspace` avec LLVM 18 : tout passe sauf
  `doc_snippets` (fa-parser), en échec à cause de la conversion en cours, non
  commitée, de la documentation vers `Array<…>`, que le parseur n'accepte pas
  encore ; la même suite passe sur la documentation de `f75ce8a`. Une exécution
  a vu échouer l'assertion temporelle native d'`async-starvation` sous charge ;
  elle passe dans l'autre exécution complète et dix fois sur dix isolée.
- Programmes du backend sur native, native sans libc et wasm, avec détection
  des fuites ; nouveaux programmes `join-length.fa` et `async-templates.fa`
  (`-O0`), `textual-templates.fa` étendu (instantanés, alias `use`, fonctions
  lisant la destination). Ces trois fichiers et `inline-strings.fa` passent aussi
  à `-O0` sur les trois cibles. Le cas d'instantané échoue à `f75ce8a`.
- Le runtime C passe ASan/UBSan, exécuté aussi directement ; l'IR versionné
  est identique à celui que régénère `generate.sh`.
- Les 25 programmes compilés avec le compilateur final donnent la sortie de
  Rust octet par octet (`blur` contrôlé à part, Goose refusant son `N`
  dynamique sous `checkfree`).

### Mesures

Médianes CPU (utilisateur + système) en secondes, passages alternés sur le
CPU 1, tailles par défaut, Goose avec `N` à l'exécution. « Avant » est
`f75ce8a`, « après » le code final. La VM était partagée avec d'autres
compilations (charge moyenne de 3 à 4) ; deux étiquettes du même binaire
`respond` ont différé de 7 % dans ces conditions. Les instructions Callgrind
(tailles réduites au dixième) sont déterministes.

| Cas | Passages | Fa avant | Fa après | Goose | Après / Goose | Instructions |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| string-building | 21 | 0,5290 | 0,1812 | 0,1669 | 1,09× | −41,2 % |
| respond | 15 | 0,6300 | 0,6571 | 0,2829 | 2,32× | +1,0 % |
| strlist | 15 | 0,3113 | 0,3186 | 0,1633 | 1,95× | −0,8 % |
| words | 15 | 0,0943 | 0,1019 | 0,0739 | 1,38× | +1,6 % |
| word-count | 15 | 0,5765 | 0,5986 | 0,4128 | 1,45× | +1,1 % |
| records | 15 | 0,3878 | 0,3817 | 0,2201 | 1,73× | +0,1 % |

`string-building` est accéléré de 2,9× ; l'écart restant avec Goose est
d'environ 9 % dans cette série (14 % dans la série du 4 novembre, 0,1632 contre
0,1438 s). Son IR optimisé est identique à celui de cette série : les
corrections de la revue ne coûtent rien à ce programme. Les petites hausses
d'instructions des autres cas viennent surtout de la branche de copie 16–32
octets de `copy_bytes` (deux instructions par appel hors ligne) ; leurs écarts
CPU restent dans la variabilité observée et ne sont pas présentés comme des
régressions ou des gains établis. Les [données](native-goose-string-building-results.json)
conservent les échantillons, empreintes des binaires et de la sortie.

### Proposition non implémentée : comptages après split

**Proposition non implémentée, à conserver pour une prochaine étape.**
À la demande de l’utilisateur, cette note ne déclenche pas son implémentation.

Une partie du coût restant vient de `split`, qui alloue et libère une copie
possédée de chaque fragment, alors que le programme ne demande que leur nombre
et la longueur de leur jointure.

La proposition consiste à éviter ce tableau intermédiaire lorsque le compilateur
prouve que le résultat de `split` ne s’échappe pas et sert uniquement à `.length`
ou à `.join(separator).length`. Ces opérations pourraient calculer les résultats
sans construire les fragments. La transformation serait générique : elle ne
dépendrait ni du nom du benchmark, ni de sa taille, ni des réponses attendues.

Cette optimisation est recommandée sous les conditions suivantes :

- Prouver tous les usages admissibles du résultat. Toute utilisation non prouvée,
  notamment une capture, un alias ou une mutation, conserve le chemin ordinaire.
- Évaluer les entrées exactement une fois et dans leur ordre initial. Conserver
  leurs valeurs par des snapshots possédés, avec les mécanismes habituels de
  nettoyage, pour que les mutations ultérieures ne changent pas le résultat.
- Respecter les règles Unicode de découpage et de comptage des graphèmes. Une
  jointure peut modifier les frontières entre fragments : conserver le chemin
  ordinaire lorsque le calcul direct ne peut pas être prouvé équivalent.
- Tester les mutations des sources et séparateurs, les captures et autres cas
  exclus, l’ordre d’évaluation, les chaînes et séparateurs vides, les frontières
  Unicode, ainsi que la durée de vie et la libération des snapshots.

Éviter une collection intermédiaire pour une requête agrégée est une optimisation
de compilateur pertinente. Elle est toutefois plus complexe que les corrections
du runtime : la sûreté dépend de la preuve des usages et du nettoyage. Elle ne
doit donc pas être présentée comme validée avant son implémentation et ses tests,
ni comme suffisante pour atteindre Goose avant de nouvelles mesures. Elle ne
nécessite pas de changer la représentation publique des tableaux ou d’introduire
des fragments empruntés dans les tableaux ordinaires.

## String-building : variante en octets

Travail du 15 novembre 2026. Objectif : que `string-building` écrit avec des
**octets** soit au moins aussi rapide que la version `String`, sans nouvelle API
du langage ni de la bibliothèque (pas de `pushAll`, de formatage vers des
octets ni de `join` d'octets). Le type `Bytes` est décidé mais pas implémenté :
les octets de longueur inconnue se construisent dans un `Array<Byte>` (stocké
de façon compacte en natif). Une seconde variante, demandée ensuite, utilise
`GrowingArray<Byte>`, dont les éléments ne bougent jamais, comme les tableaux
`u8[>..]` de Goose qui vivent dans une région réservée.

### Les programmes

- `benchmarks/string-building/bytes.fa` reprend l'algorithme du portage Goose :
  tampons d'octets extensibles, `split` en intervalles `(début, fin)` du texte
  (les tranches de Goose, sans copie), `join` dans un nouveau tampon. Les
  entiers sont formatés à la main : nombre de chiffres compté par comparaisons,
  chiffres écrits depuis la fin deux par deux avec une table `00`…`99`. Le texte
  du dernier élément est vérifié contre une chaîne construite comme dans
  `main.fa`. Ces choix de portage sont ceux d'un utilisateur attentif : ils ne
  sautent aucun travail fait par Goose (toutes les parties sont calculées et la
  jointure est construite).
- `benchmarks/string-building/growing.fa` est le même programme avec des
  tampons `GrowingArray<Byte>`. `GrowingArray` n'a pas de `clear()` : le portage
  affecte `[]`, la façon documentée de supprimer tous les éléments.
- `compare.py` mesure ces variantes comme `string-building-bytes` et
  `string-building-growing`, contre les mêmes programmes Goose et Rust, aux
  mêmes tailles (`VARIANTS`). `variants.py` alterne plusieurs exécutables
  fixés sur un CPU et compte leurs instructions.

### Optimisations du compilateur et du runtime

Toutes sont génériques : elles portent sur la forme des boucles, les types
des éléments et les règles d'alias, jamais sur un benchmark, une taille ou une
sortie.

- **Copie par blocs** (`lower/copies.rs`, `copy_loop`). `for x in source {
  cible.push(x) }` et `for i in a..b { cible.push(source[i]) }` (avec ou sans
  `try`, `..=` compris) deviennent un seul `fa_array_extend` : place réservée
  une fois avec une croissance géométrique, puis une copie. Conditions : le
  corps est ce seul `push` ; son argument n'a aucune conversion au puits (ni
  contrôle d'intervalle ni changement de représentation) ; les éléments ne
  possèdent pas de mémoire ; source et cible sont lues depuis un local, un
  paramètre et leurs champs ordinaires (aucun getter, index ni appel) ; rien
  n'est réactif ; la fonction n'est pas une coroutine (ses boucles sont des
  points coopératifs). Source et cible sont distinctes : l'analyseur refuse un
  `push` dont l'argument est dans la cible (REF013) et la modification du
  tableau parcouru (REF012), testé par `ref-44.fa`. La copie n'en dépend pas
  pour autant : la place est faite avant de lire les éléments de la source,
  depuis le bloc qu'elle a alors.
- **Contrôle d'intervalle sorti de la boucle.** Une plage n'est copiée que si
  `0 <= a <= b <= source.length` (`b < length` pour `a..=b`), vérifié une fois.
  Sinon la boucle s'exécute telle qu'écrite : l'`IndexError` survient au même
  tour, après les mêmes `push`, et un indice négatif compte depuis la fin.
- **Remplissage.** `for _ in a..b { cible.push(octet) }` sur des octets devient
  `fa_array_fill` (quelques écritures bornées pour les petites longueurs).
- **Recherche.** Une boucle sur des octets dont le corps est seulement
  `if octet == constante { … }` va d'une occurrence à la suivante avec
  `fa_bytes_next`, seize octets à la fois. Les tours sautés ne font rien ; la
  recherche s'exécute après le corps de l'occurrence précédente et lit donc ce
  que ces tours auraient lu (une écriture par la variable de boucle compris).
- **`push` en ligne.** Le chemin courant est écrit dans l'IR avec les étiquettes
  TBAA (`fa header` pour l'en-tête, le type scalaire pour l'élément), et la
  croissance passe par `fa_array_grow`, froid et hors ligne. Après cet appel,
  le slot du tableau (local ou paramètre et champs) est relu et fusionné par
  un `phi` : LLVM garde alors l'en-tête, la longueur, la capacité et les
  éléments en registres d'un `push` à l'autre, au lieu de les relire en
  mémoire à chaque fois (une chaîne de dépendances par la mémoire).
- **Réservation avant une boucle comptée** (`reserve_for_pushes`). Elle accepte
  les paramètres `mutates` (une racine distincte dans la fonction, d'après les
  règles d'alias) et réserve de façon géométrique (`fa_array_room`), ce qui
  supprime une réallocation par tour d'une boucle englobante.
- **Correctif.** Un local de module ou capturé, réservé avant une boucle dont le
  corps appelait une fonction capable de le remplacer (`xs = []`), recevait
  ensuite des `push` sans contrôle de capacité dans le nouveau bloc : écriture
  hors bornes, présente avant ce travail. Un tel local n'est plus réservé que
  si le corps n'appelle rien d'autre que ce `push`.

Pour `GrowingArray` (liste segmentée, décision « GrowingArray native layout ») :

- **Curseur.** L'en-tête porte l'adresse de l'élément `length` et la fin de son
  segment : un `push` est une comparaison, une écriture et un incrément ; le
  segment suivant est trouvé ou alloué hors ligne (`fa_segmented_next`).
- **Parcours par segment.** Une boucle `for` avance un pointeur dans le segment
  et ne cherche le segment suivant qu'à sa fin (`fa_segmented_run`), au lieu
  d'un `clz` par élément. La recherche d'octets continue depuis ce curseur.
- **Copies, remplissages et recherches** de `copies.rs` acceptent un
  `GrowingArray` en source ou en cible, avec une copie par segment
  (`fa_segmented_append`, `fa_segmented_extend_to`, `fa_segmented_fill`) et un
  chemin en ligne quand tout tient dans un segment.
- **Indexation.** L'adresse d'un élément est calculée dans l'IR avec
  l'étiquette d'en-tête (un `store` dans un élément ne fait plus relire le
  pointeur de segment), et le contrôle `0 <= index < length` vient d'abord,
  l'indice négatif étant traité dans un bloc froid, comme pour `Array`.
- **`x = []`** sur un `GrowingArray` d'un local ou d'un champ supprime les
  éléments (ceux qui possèdent de la mémoire sont libérés) et garde les
  segments pour les `push` suivants (`fa_segmented_reset`). Les références dans
  l'ancienne valeur sont déjà invalides.

### Mesures

Méthode : `compare.py --default-sizes --goose-runtime-size` construit les
trois programmes Fa (`string-building`, `-bytes`, `-growing`), Goose avec `N`
lu à l'exécution, et vérifie chaque sortie contre Rust. `variants.py` alterne
ensuite les quatre exécutables, processus fixé au CPU 1 : trois séries de 21
passages à 3 000 000 entrées, médiane du temps CPU utilisateur + système. Les
instructions sont celles de Callgrind (processus entier) à 300 000 entrées. La
VM restait partagée (charge moyenne de 1 à 1,4 pendant ces séries, de 4 à 6
pendant le développement) ; les compteurs matériels sont inaccessibles
(`perf_event_paranoid` = 4).

| Version | Médiane CPU (63 passages) | Médianes des séries | Instructions |
| --- | ---: | ---: | ---: |
| `String` (`main.fa`) | 0,198 s | 0,201 / 0,192 / 0,202 | 174,1 M |
| `Array<Byte>` (`bytes.fa`) | 0,191 s | 0,190 / 0,204 / 0,189 | 190,3 M |
| `GrowingArray<Byte>` (`growing.fa`) | 0,230 s | 0,229 / 0,227 / 0,238 | 206,5 M |
| Goose | 0,164 s | 0,160 / 0,180 / 0,171 | 148,7 M |

Au départ, la première version du portage en octets prenait 0,540 s et
580,0 M instructions, et la version `GrowingArray` 1 195,8 M instructions.

- **`Array<Byte>` atteint la parité avec `String`** : médiane groupée 0,965×,
  plus basse dans deux séries sur trois, minimum plus bas dans les trois. La
  marge est mince : sur les 63 tours appariés, `Array<Byte>` est le plus rapide
  dans 32. Il exécute 9 % d'instructions de plus que `String` avec un meilleur
  débit (des copies et comparaisons d'octets simples, là où `String` passe par
  le runtime des chaînes). La version `String` ne construit pas la jointure
  (`join(…).length` calcule seulement sa longueur) ; le portage la construit,
  comme Goose.
- **`GrowingArray<Byte>` vaut 1,20× `Array<Byte>` en temps** (1,085× en
  instructions) et reste plus lent que Goose : le seuil de 1,10× n'est atteint
  qu'en instructions. L'écart, à 300 000 entrées : `appendInteger` +12,3 M
  (chaque écriture indexée teste le premier segment et charge le pointeur de
  segment ; le remplissage vérifie la place du segment courant), `add` +13,0 M
  (la position du segment de début de chaque copie, un `clz` et des décalages,
  et la recherche par segment), `main` +3,0 M (la remise à zéro par `[]`, un
  appel par entrée) ; en sens inverse, aucune recopie lors de la croissance
  (`memcpy` −12,8 M). Que l'écart en temps dépasse l'écart en instructions
  désigne des chaînes de dépendances plus longues (calcul du segment avant
  chaque accès, longueur et curseur écrits à chaque `push`) ; sans compteurs
  matériels, cette attribution n'est pas mesurée. Les pistes : une taille de
  premier segment propre au tableau (`k` avec `capacity`/`reserve`, hors du
  périmètre de ce travail selon `design/todo.md`), un en-tête plus petit, et
  un accès au segment du curseur sans `clz`.
- Aucun programme du benchmark n'appelle une méthode qui aplatit un
  `GrowingArray` (`fa_segmented_flatten`) : seuls `length`, `isEmpty`,
  l'indexation, `push` et `for` sont utilisés.

**Contrôle des autres programmes.** Les 25 programmes, compilés avant
(`f1317521`) et après, aux tailles d'audit de `compare.py`, avec les mêmes
binaires Goose et Rust : toutes les sorties sont identiques à celles de Rust et
d'avant (`blur` vérifié à part, Goose refusant son `N` à l'exécution sous
`checkfree` : 96 692 261 puis 96 692 230 instructions). Dix exécutables sont
identiques octet par octet. Gains : `records` −1,9 %, `respond` −1,8 %,
`graph` −1,4 %, `lru` −0,7 %, `fannkuch-redux` −0,1 %. Hausses : `scene`
+0,63 % (la réservation géométrique alloue des blocs plus grands :
`array_extend_room` et `heap_alloc_slow`, alors que `build` lui-même baisse),
`words` +0,30 % (le code des `push` écrits en ligne dans `main`),
`sort-objects` +0,06 %. Les autres sont à ±600 instructions près. Les
[données](native-goose-bytes-results.json) conservent les échantillons, les
empreintes et les comptes avant et après.

### Validation

Avec `LLVM_SYS_181_PREFIX`, `CARGO_TARGET_DIR=$HOME/.cache/fa-goose-target` et
`CARGO_BUILD_JOBS=4` :

- `cargo fmt --all` ; `cargo clippy --workspace --all-targets` sans
  avertissement ;
- `cargo test --workspace --no-fail-fast` : tout passe (1 089 tests dans 53
  suites), dont les programmes du backend sur native, native sans libc et wasm
  avec détection des fuites, `doc_snippets` et les cas de l'analyseur. Dans
  une exécution, l'assertion temporelle d'`async-starvation` a échoué sur wasm
  sous charge ; isolée, la suite des programmes passe trois fois sur trois ;
- nouveaux tests : `block-copies.fa` et `growing-blocks.fa` (sorties
  identiques au backend JavaScript), leurs versions `-O0`, quatre tests d'IR
  (copie en un bloc, contrôle sorti de la boucle avec les tours conservés,
  remplissage et recherche, en-tête en registres) plus un pour le `push` d'un
  `GrowingArray`, et le cas d'analyseur `ref-44.fa` ;
- le runtime C est régénéré par `generate.sh` (IR versionné identique à celui
  qu'il produit) ; le `fa` de release est reconstruit avant les mesures.


## Capacité et réservation (2026-11-15)

`capacity`, `reserve(additional)` et `from(source, capacity = n)` sont disponibles
sur les chaînes et toutes les collections redimensionnables, dont toutes les
variantes croissantes et `OrderedColumnar`. `Bytes` reste de longueur fixe.
L'en-tête de `GrowingArray` passe à 64 octets (40 sur wasm32), avec trois
pointeurs de segments intégrés ; `k` appartient au tableau et une réservation à
vide donne un premier bloc contigu. Les adresses des éléments existants restent
stables. Les conventions de retour des fonctions ne changent pas.

Les portages en octets réservent `parts`, `joined`, `output` et `lastEntry`.
`main.fa` réserve aussi `output` : c'est l'usage naturel de l'API pour un bloc
borné de lignes, dont le stockage est réutilisé entre les blocs.

Mesure : `variants.py`, 21 passages alternés, CPU 1, médiane utilisateur + système,
3 000 000 entrées ; Callgrind sur le processus entier à 300 000 entrées.
Goose est compilé avec **clang 18, `-O2`**, vérifié dans les journaux de construction
(`GOOSE_CC` et `CC` explicitement définis). VM partagée : charge moyenne de 9,5
à la collecte finale ; les temps avant/après ne sont pas appariés. Les
[données brutes](native-goose-capacity-results.json) gardent les échantillons,
les commandes, les empreintes des exécutables et les vérifications contre Rust.

| Version | Médiane CPU | Instructions | Variation des instructions depuis `c74a2278` |
| --- | ---: | ---: | ---: |
| `String` | 0,1810 s | 174,412 M | +0,15 % |
| `Array<Byte>` | 0,1968 s | 177,365 M | −6,80 % |
| `GrowingArray<Byte>` | 0,2136 s | 224,643 M | +8,79 % |
| Goose | 0,1557 s | 148,698 M | stable |

La réservation réduit les croissances de l'`Array`. Le tableau croissant vaut
1,086 fois l'`Array` en temps, mais reste à 1,372 fois Goose : le critère de
clôture de la décision n'est donc pas atteint. Sa hausse d'instructions est
réelle : `appendInteger` +11,47 M, `main` +5,73 M et `add` +2,87 M, partiellement
compensées par la disparition des allocations de croissance. L'assembleur montre
le coût du `k` variable, des choix de segment et des chargements d'en-tête ; sa
lecture avant les contrôles de bornes permet à LLVM de la sortir de la boucle
d'écriture des chiffres. Il reste un travail de performance ; cette mesure ne
justifie pas à elle seule une réservation d'espace virtuel.

**Contrôle des 25 programmes** : toutes les sorties restent identiques à Rust.
Les variations au-delà des quelques instructions de démarrage sont : `calc`
+0,068 %, `records` +0,149 %, `respond` +0,256 %, `sexp` −0,052 %,
`string-building` +0,151 %, `word-count` +0,406 % et `words` −0,257 %.
Les 18 autres restent à 113 instructions près. Les correctifs finaux de libération
des sources lorsqu'une expression de capacité lève une erreur, et du chemin
JavaScript des tableaux de booléens, produisent les mêmes 27 exécutables natifs
d'audit, octet pour octet, que ceux mesurés (liaison avec `CC=clang18`).


Un contrôle supplémentaire conserve un `GrowingArray<Integer>` **sans
réservation** : un million d'éléments, dix millions de lectures à pas 997,
21 passages alternés. Médianes CPU avant/après : 0,1331 / 0,1378 s ; médiane des
rapports appariés : 1,0054 (nouveau plus rapide dans 10 paires sur 21).
Callgrind, avec 100 000 lectures après la même initialisation : +300 139
instructions (+2,08 %). Le curseur de `push` conserve les quatre chargements
d'en-tête vérifiés par le test d'IR ; l'indexation à `k` variable garde un coût.

Validation finale : 1 090 tests Rust dans 53 suites, zéro échec, huit ignorés ;
Clippy sans avertissement ; 514 tests du runtime JavaScript ; documentation,
construction du site et tests C avec ASan/UBSan. Les programmes s'exécutent sur
native, freestanding et wasm avec détection des fuites, y compris à `-O0`.
