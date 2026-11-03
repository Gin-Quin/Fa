# Rapport intermédiaire : performances natives de Fa face à Goose

Analyse du 3 novembre 2026, branche `perf/native-goose`, départ
`9c3032aa3eb4d8dced7cb94f7aeb99d0624e6a9c`.
Dernier code mesuré : `6067418dbe9db284956377b75d2d653b41d9712f`.
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
40 octets par élément de `Array(Event)` réduisait les instructions de `records`
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
  `Array(String).join` calcule également la taille finale avant la copie.
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
