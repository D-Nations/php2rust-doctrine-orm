//! Upstream: doctrine/inflector src/Rules/Portuguese/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\Portuguese\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/^(g|)ases$/i")?, r"\1ás"),
            Transformation::new(
                Pattern::new("/(japon|escoc|ingl|dinamarqu|fregu|portugu)eses$/i")?,
                r"\1ês",
            ),
            Transformation::new(Pattern::new("/(ae|ao|oe)s$/")?, "ao"),
            Transformation::new(Pattern::new("/(ãe|ão|õe)s$/")?, "ão"),
            Transformation::new(Pattern::new("/^(.*[^s]s)es$/i")?, r"\1"),
            Transformation::new(Pattern::new("/sses$/i")?, "sse"),
            Transformation::new(Pattern::new("/ns$/i")?, "m"),
            Transformation::new(Pattern::new("/(r|t|f|v)is$/i")?, r"\1il"),
            Transformation::new(Pattern::new("/uis$/i")?, "ul"),
            Transformation::new(Pattern::new("/ois$/i")?, "ol"),
            Transformation::new(Pattern::new("/eis$/i")?, "ei"),
            Transformation::new(Pattern::new("/éis$/i")?, "el"),
            Transformation::new(Pattern::new("/([^p])ais$/i")?, r"\1al"),
            Transformation::new(Pattern::new("/(r|z)es$/i")?, r"\1"),
            Transformation::new(Pattern::new("/^(á|gá)s$/i")?, r"\1s"),
            Transformation::new(Pattern::new("/([^ê])s$/i")?, r"\1"),
        ])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/^(alem|c|p)ao$/i")?, r"\1aes"),
            Transformation::new(Pattern::new("/^(irm|m)ao$/i")?, r"\1aos"),
            Transformation::new(Pattern::new("/ao$/i")?, "oes"),
            Transformation::new(Pattern::new("/^(alem|c|p)ão$/i")?, r"\1ães"),
            Transformation::new(Pattern::new("/^(irm|m)ão$/i")?, r"\1ãos"),
            Transformation::new(Pattern::new("/ão$/i")?, "ões"),
            Transformation::new(Pattern::new("/^(|g)ás$/i")?, r"\1ases"),
            Transformation::new(
                Pattern::new("/^(japon|escoc|ingl|dinamarqu|fregu|portugu)ês$/i")?,
                r"\1eses",
            ),
            Transformation::new(Pattern::new("/m$/i")?, "ns"),
            Transformation::new(Pattern::new("/([^aeou])il$/i")?, r"\1is"),
            Transformation::new(Pattern::new("/ul$/i")?, "uis"),
            Transformation::new(Pattern::new("/ol$/i")?, "ois"),
            Transformation::new(Pattern::new("/el$/i")?, "eis"),
            Transformation::new(Pattern::new("/al$/i")?, "ais"),
            Transformation::new(Pattern::new("/(z|r)$/i")?, r"\1es"),
            Transformation::new(Pattern::new("/(s)$/i")?, r"\1"),
            Transformation::new(Pattern::new("/$/")?, "s"),
        ])
    }

    pub fn get_irregular() -> Vec<Substitution> {
        vec![
            Substitution::new(Word::new("abdomen"), Word::new("abdomens")),
            Substitution::new(Word::new("alemão"), Word::new("alemães")),
            Substitution::new(Word::new("artesã"), Word::new("artesãos")),
            Substitution::new(Word::new("álcool"), Word::new("álcoois")),
            Substitution::new(Word::new("árvore"), Word::new("árvores")),
            Substitution::new(Word::new("bencão"), Word::new("bencãos")),
            Substitution::new(Word::new("cão"), Word::new("cães")),
            Substitution::new(Word::new("campus"), Word::new("campi")),
            Substitution::new(Word::new("cadáver"), Word::new("cadáveres")),
            Substitution::new(Word::new("capelão"), Word::new("capelães")),
            Substitution::new(Word::new("capitão"), Word::new("capitães")),
            Substitution::new(Word::new("chão"), Word::new("chãos")),
            Substitution::new(Word::new("charlatão"), Word::new("charlatães")),
            Substitution::new(Word::new("cidadão"), Word::new("cidadãos")),
            Substitution::new(Word::new("consul"), Word::new("consules")),
            Substitution::new(Word::new("cristão"), Word::new("cristãos")),
            Substitution::new(Word::new("difícil"), Word::new("difíceis")),
            Substitution::new(Word::new("email"), Word::new("emails")),
            Substitution::new(Word::new("escrivão"), Word::new("escrivães")),
            Substitution::new(Word::new("fóssil"), Word::new("fósseis")),
            Substitution::new(Word::new("gás"), Word::new("gases")),
            Substitution::new(Word::new("germens"), Word::new("germen")),
            Substitution::new(Word::new("grão"), Word::new("grãos")),
            Substitution::new(Word::new("hífen"), Word::new("hífens")),
            Substitution::new(Word::new("irmão"), Word::new("irmãos")),
            Substitution::new(Word::new("liquens"), Word::new("liquen")),
            Substitution::new(Word::new("mal"), Word::new("males")),
            Substitution::new(Word::new("mão"), Word::new("mãos")),
            Substitution::new(Word::new("orfão"), Word::new("orfãos")),
            Substitution::new(Word::new("país"), Word::new("países")),
            Substitution::new(Word::new("pai"), Word::new("pais")),
            Substitution::new(Word::new("pão"), Word::new("pães")),
            Substitution::new(Word::new("projétil"), Word::new("projéteis")),
            Substitution::new(Word::new("réptil"), Word::new("répteis")),
            Substitution::new(Word::new("sacristão"), Word::new("sacristães")),
            Substitution::new(Word::new("sotão"), Word::new("sotãos")),
            Substitution::new(Word::new("tabelião"), Word::new("tabeliães")),
        ]
    }
}
