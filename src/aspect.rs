use strsim::normalized_levenshtein;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Aspect {
    Aer,
    Alienis,
    Aqua,
    Arbor,
    Auram,
    Bestia,
    Caelum,
    Cognitio,
    Corpus,
    Desidia,
    Electrum,
    Exanimis,
    Fabrico,
    Fames,
    Gelum,
    Gula,
    Herba,
    Humanus,
    Ignis,
    Infernus,
    Instrumentum,
    Invidia,
    Ira,
    Iter,
    Limus,
    Lucrum,
    Lux,
    Luxuria,
    Machina,
    Magneto,
    Messis,
    Metallum,
    Meto,
    Mortuus,
    Motus,
    Nebrisum,
    Ordo,
    Pannus,
    Perditio,
    Perfodio,
    Permutatio,
    Potentia,
    Praecantatio,
    Radio,
    Sano,
    Sensus,
    Spiritus,
    Strontio,
    Superbia,
    Tabernus,
    Telum,
    Tempestas,
    Tempus,
    Tenebrae,
    Terra,
    Tutamen,
    Vacuos,
    Venenum,
    Victus,
    Vinculum,
    Vitium,
    Vitreus,
    Volatus,
}

impl Aspect {
    fn values() -> &'static [Aspect] {
        static VALUES: [Aspect; 63] = [
            Aspect::Aer,
            Aspect::Alienis,
            Aspect::Aqua,
            Aspect::Arbor,
            Aspect::Auram,
            Aspect::Bestia,
            Aspect::Caelum,
            Aspect::Cognitio,
            Aspect::Corpus,
            Aspect::Desidia,
            Aspect::Electrum,
            Aspect::Exanimis,
            Aspect::Fabrico,
            Aspect::Fames,
            Aspect::Gelum,
            Aspect::Gula,
            Aspect::Herba,
            Aspect::Humanus,
            Aspect::Ignis,
            Aspect::Infernus,
            Aspect::Instrumentum,
            Aspect::Invidia,
            Aspect::Ira,
            Aspect::Iter,
            Aspect::Limus,
            Aspect::Lucrum,
            Aspect::Lux,
            Aspect::Luxuria,
            Aspect::Machina,
            Aspect::Magneto,
            Aspect::Messis,
            Aspect::Metallum,
            Aspect::Meto,
            Aspect::Mortuus,
            Aspect::Motus,
            Aspect::Nebrisum,
            Aspect::Ordo,
            Aspect::Pannus,
            Aspect::Perditio,
            Aspect::Perfodio,
            Aspect::Permutatio,
            Aspect::Potentia,
            Aspect::Praecantatio,
            Aspect::Radio,
            Aspect::Sano,
            Aspect::Sensus,
            Aspect::Spiritus,
            Aspect::Strontio,
            Aspect::Superbia,
            Aspect::Tabernus,
            Aspect::Telum,
            Aspect::Tempestas,
            Aspect::Tempus,
            Aspect::Tenebrae,
            Aspect::Terra,
            Aspect::Tutamen,
            Aspect::Vacuos,
            Aspect::Venenum,
            Aspect::Victus,
            Aspect::Vinculum,
            Aspect::Vitium,
            Aspect::Vitreus,
            Aspect::Volatus,
        ];
        &VALUES
    }

    pub fn from_str_fuzzy(name: &String) -> Option<(Aspect, f64)> {
        let mut highest_score = 0.0;
        let mut best_match = None;

        for variant in Aspect::values().iter() {
            let variant_name = format!("{:?}", variant).to_lowercase();
            let input_name = name.to_lowercase();
            let score = normalized_levenshtein(&variant_name, &input_name);

            if score > highest_score {
                highest_score = score;
                best_match = Some(variant.clone());
            }
        }
        if best_match.is_some() {
            Some((best_match.unwrap(), highest_score))
        } else {
            None
        }
    }
}
