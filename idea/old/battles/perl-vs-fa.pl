# PERL (496 characters) ~ 30% larger
sub lire_ini {
  my $fichier = shift or die;
  my $ini = lire_fichier_ini( $fichier );
  my %champs;

  foreach my $compte(keys %$ini) {
    next unless $compte =~m/\./;

    my $section = $ini->{$compte};
    my @champs = ("FDS", "FICHIER", "NOM", "DATE", "VERSION");
    my $nb = $section->{'nb_champs'};

    for my $n ( 1..$nb ) {
      my $code = $section->{"champ_{n}_code"};
      push( @champs, $code );
    }

    $champs{$compte} = [ map { lc($_) } @champs ];
  }

  return \%champs;
}

# FA (383 characters)
let lire_ini(fichier: String) -> champs = empty [String]
  let ini = lire_fichier_ini(fichier)

  for compte in ini.keys()
    if no compte ~= ~m/\./: next

    let section = ini[compte]
    let champs = ['FDS', 'FICHIER', 'NOM', 'DATE', 'VERSION']
    let section >> { nb_champs }

    for n in 1..nb_champs
      champs += section["champ_{n}_code"]

    champs[compte] = champs.map String.lowerCase


# JAVASCRIPT (435 characters) ~ 15% larger
lire_ini(fichier) {
  let champs = []
  let ini = lire_fichier_ini(fichier)

  for (let compte of ini.keys()) {
    if (!(~m/\./.test(compte)))
      continue

    let section = ini[compte]
    let champs = ['FDS', 'FICHIER', 'NOM', 'DATE', 'VERSION']
    let nb = section.nb_champs

    for (let n=0; n < nb; n++)
      champs.push(section[`champ_${n}_code`])

    champs[compte] = champs.map(String.lowerCase)
    return champs
  }
}
