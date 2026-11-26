//! Upstream: doctrine/inflector src/Rules/Italian/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\Italian\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            // Reverse of -sce → -scia (fasce → fascia)
            Transformation::new(Pattern::new("([aeiou])sce$")?, r"\1scia"),
            // Reverse of -cie → -cia (farmacia → farmacie)
            Transformation::new(Pattern::new("cie$")?, "cia"),
            // Reverse of -gie → -gia (bugia → bugie)
            Transformation::new(Pattern::new("gie$")?, "gia"),
            // Reverse of -ce → -cia (arance → arancia)
            Transformation::new(Pattern::new("([^aeiou])ce$")?, r"\1cia"),
            // Reverse of -ge → -gia (valige → valigia)
            Transformation::new(Pattern::new("([^aeiou])ge$")?, r"\1gia"),
            // Reverse of -chi → -co (bachi → baco)
            Transformation::new(
                Pattern::new("([bcdfghjklmnpqrstvwxyz][aeiou])chi$")?,
                r"\1co",
            ),
            // Reverse of -ghi → -go (laghi → lago)
            Transformation::new(
                Pattern::new("([bcdfghjklmnpqrstvwxyz][aeiou])ghi$")?,
                r"\1go",
            ),
            // Reverse of -ci → -co (medici → medico)
            Transformation::new(
                Pattern::new("([aeiou][bcdfghjklmnpqrstvwxyz])ci$")?,
                r"\1co",
            ),
            // Reverse of -gi → -go (psicologi → psicologo)
            Transformation::new(
                Pattern::new("([aeiou][bcdfghjklmnpqrstvwxyz])gi$")?,
                r"\1go",
            ),
            // Reverse of -i → -io (zii → zio, negozi → negozio)
            // This is more complex due to Italian's stress patterns, but we'll handle the basic case
            Transformation::new(Pattern::new("([^aeiou])i$")?, r"\1io"),
            // Handle words that end with -i but should go to -co/-go (amici → amico, not amice)
            Transformation::new(Pattern::new("([^aeiou])ci$")?, r"\1co"),
            Transformation::new(Pattern::new("([^aeiou])gi$")?, r"\1go"),
            // Reverse of -a → -e
            Transformation::new(Pattern::new("e$")?, "a"),
            // Reverse of -e → -i
            Transformation::new(Pattern::new("i$")?, "e"),
            // Reverse of -o → -i
            Transformation::new(Pattern::new("i$")?, "o"),
        ])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            // Words ending in -scia without stress on 'i' become -sce (e.g. fascia → fasce)
            Transformation::new(Pattern::new("([aeiou])scia$")?, r"\1sce"),
            // Words ending in -cia/gia with stress on 'i' keep the 'i' in plural
            Transformation::new(Pattern::new("cia$")?, "cie"), // e.g. farmacia → farmacie
            Transformation::new(Pattern::new("gia$")?, "gie"), // e.g. bugia → bugie
            // Words ending in -cia/gia without stress on 'i' lose the 'i' in plural
            Transformation::new(Pattern::new("([^aeiou])cia$")?, r"\1ce"), // e.g. arancia → arance
            Transformation::new(Pattern::new("([^aeiou])gia$")?, r"\1ge"), // e.g. valigia → valige
            // Words ending in -co/-go with stress on 'o' become -chi/-ghi
            Transformation::new(
                Pattern::new("([bcdfghjklmnpqrstvwxyz][aeiou])co$")?,
                r"\1chi",
            ), // e.g. baco → bachi
            Transformation::new(
                Pattern::new("([bcdfghjklmnpqrstvwxyz][aeiou])go$")?,
                r"\1ghi",
            ), // e.g. lago → laghi
            // Words ending in -co/-go with stress on the penultimate syllable become -ci/-gi
            Transformation::new(
                Pattern::new("([aeiou][bcdfghjklmnpqrstvwxyz])co$")?,
                r"\1ci",
            ), // e.g. medico → medici
            Transformation::new(
                Pattern::new("([aeiou][bcdfghjklmnpqrstvwxyz])go$")?,
                r"\1gi",
            ), // e.g. psicologo → psicologi
            // Words ending in -io with stress on 'i' keep the 'i' in plural
            Transformation::new(Pattern::new("([^aeiou])io$")?, r"\1i"), // e.g. zio → zii
            // Words ending in -io with stress on 'o' lose the 'i' in plural
            Transformation::new(Pattern::new("([aeiou])io$")?, r"\1i"), // e.g. negozio → negozi
            // Standard ending rules
            Transformation::new(Pattern::new("a$")?, "e"), // -a → -e
            Transformation::new(Pattern::new("e$")?, "i"), // -e → -i
            Transformation::new(Pattern::new("o$")?, "i"), // -o → -i
        ])
    }

    #[allow(clippy::too_many_lines, reason = "upstream's word list")]
    pub fn get_irregular() -> Vec<Substitution> {
        // Irregular substitutions (singular => plural)
        let irregulars: &[(&str, &str)] = &[
            ("ala", "ali"),
            ("albergo", "alberghi"),
            ("amica", "amiche"),
            ("amico", "amici"),
            ("ampio", "ampi"),
            ("arancia", "arance"),
            ("arma", "armi"),
            ("asparago", "asparagi"),
            ("banca", "banche"),
            ("belga", "belgi"),
            ("braccio", "braccia"),
            ("budello", "budella"),
            ("bue", "buoi"),
            ("caccia", "cacce"),
            ("calcagno", "calcagna"),
            ("camicia", "camicie"),
            ("cane", "cani"),
            ("capitale", "capitali"),
            ("carcere", "carceri"),
            ("casa", "case"),
            ("cavaliere", "cavalieri"),
            ("centinaio", "centinaia"),
            ("cerchio", "cerchia"),
            ("cervello", "cervella"),
            ("chiave", "chiavi"),
            ("chirurgo", "chirurgi"),
            ("ciglio", "ciglia"),
            ("città", "città"),
            ("corno", "corna"),
            ("corpo", "corpi"),
            ("crisi", "crisi"),
            ("dente", "denti"),
            ("dio", "dei"),
            ("dito", "dita"),
            ("dottore", "dottori"),
            ("fiore", "fiori"),
            ("fratello", "fratelli"),
            ("fuoco", "fuochi"),
            ("gamba", "gambe"),
            ("ginocchio", "ginocchia"),
            ("gioco", "giochi"),
            ("giornale", "giornali"),
            ("giraffa", "giraffe"),
            ("labbro", "labbra"),
            ("lenzuolo", "lenzuola"),
            ("libro", "libri"),
            ("madre", "madri"),
            ("maestro", "maestri"),
            ("magico", "magici"),
            ("mago", "maghi"),
            ("maniaco", "maniaci"),
            ("manico", "manici"),
            ("mano", "mani"),
            ("medico", "medici"),
            ("membro", "membri"),
            ("metropoli", "metropoli"),
            ("migliaio", "migliaia"),
            ("miglio", "miglia"),
            ("mille", "mila"),
            ("mio", "miei"),
            ("moglie", "mogli"),
            ("mosaico", "mosaici"),
            ("muro", "muri"),
            ("nemico", "nemici"),
            ("nome", "nomi"),
            ("occhio", "occhi"),
            ("orecchio", "orecchi"),
            ("osso", "ossa"),
            ("paio", "paia"),
            ("pane", "pani"),
            ("papa", "papi"),
            ("pasta", "paste"),
            ("penna", "penne"),
            ("pesce", "pesci"),
            ("piede", "piedi"),
            ("pittore", "pittori"),
            ("poeta", "poeti"),
            ("porco", "porci"),
            ("porto", "porti"),
            ("problema", "problemi"),
            ("ragazzo", "ragazzi"),
            ("re", "re"),
            ("rene", "reni"),
            ("riso", "risa"),
            ("rosa", "rosa"),
            ("sale", "sali"),
            ("sarto", "sarti"),
            ("scuola", "scuole"),
            ("serie", "serie"),
            ("serramento", "serramenta"),
            ("sorella", "sorelle"),
            ("specie", "specie"),
            ("staio", "staia"),
            ("stazione", "stazioni"),
            ("strido", "strida"),
            ("strillo", "strilla"),
            ("studio", "studi"),
            ("suo", "suoi"),
            ("superficie", "superfici"),
            ("tavolo", "tavoli"),
            ("tempio", "templi"),
            ("treno", "treni"),
            ("tuo", "tuoi"),
            ("uomo", "uomini"),
            ("uovo", "uova"),
            ("urlo", "urla"),
            ("valigia", "valigie"),
            ("vestigio", "vestigia"),
            ("vino", "vini"),
            ("viola", "viola"),
            ("zio", "zii"),
        ];

        irregulars
            .iter()
            .map(|(singular, plural)| Substitution::new(Word::new(*singular), Word::new(*plural)))
            .collect()
    }
}
