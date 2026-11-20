//! Mode simple de l'écran Règles : logique pure (compatibilité, raccourcis, étapes, phrase récapitulative).
//! L'affichage de l'assistant est dans la suite de ce fichier ; ici rien ne dépend d'egui, donc tout se teste.

use crate::engine::{Action, ActionKind, Condition, Field, Folder, Op, Rule};
use crate::mechanics::{parse_size, parse_span};
use crate::tags;

/// Étape de l'assistant : 1 = dossiers, 2 = conditions, 3 = action.
pub const LAST_STEP: u8 = 3;

/// Actions proposées en mode simple : celles qui ne détruisent rien de façon irréversible.
pub const SIMPLE_ACTIONS: &[ActionKind] =
    &[ActionKind::Move, ActionKind::Copy, ActionKind::Rename, ActionKind::Trash, ActionKind::AddTag, ActionKind::Compress];

/// Raccourci de condition : un libellé de carte et la condition qu'il ajoute.
pub struct Preset {
    pub label: &'static str,
    pub make: fn() -> Condition,
}

fn cond(field: Field, op: Op, value: &str) -> Condition {
    Condition { field, op, value: value.into() }
}

pub const PRESETS: &[Preset] = &[
    Preset { label: "C'est une image", make: || cond(Field::Kind, Op::Is, "image") },
    Preset { label: "C'est un PDF", make: || cond(Field::Extension, Op::Is, "pdf") },
    Preset { label: "C'est une vidéo", make: || cond(Field::Kind, Op::Is, "video") },
    Preset { label: "C'est un document", make: || cond(Field::Kind, Op::Is, "document") },
    Preset { label: "Plus ancien que 30 jours", make: || cond(Field::DateModified, Op::NotWithinLast, "30 days") },
    Preset { label: "Le nom contient…", make: || cond(Field::Name, Op::Contains, "") },
    Preset { label: "Plus gros que…", make: || cond(Field::Size, Op::Greater, "100 MB") },
];

/// Opérateurs proposés en mode simple : tout sauf l'expression régulière (« correspond à »).
pub fn simple_ops(field: Field) -> Vec<Op> {
    field.ops().iter().copied().filter(|op| *op != Op::Matches).collect()
}

/// La règle tient-elle dans l'assistant ? Sinon on la laisse intacte et on propose le mode avancé.
pub fn is_simple_compatible(rule: &Rule) -> bool {
    let one_condition = rule.conditions.len() <= 1;
    rule.recursion_depth == 0
        && (rule.match_all || one_condition)
        && rule.conditions.iter().all(|c| c.op != Op::Matches)
        && rule.actions.len() <= 1
        && rule.actions.iter().all(|a| SIMPLE_ACTIONS.contains(&a.kind))
}

/// L'étape est-elle complète ? Sinon : la phrase qui dit quoi faire.
pub fn step_ready(step: u8, rule: &Rule) -> Result<(), &'static str> {
    match step {
        1 if rule.folder_ids.is_empty() => Err("Choisissez au moins un dossier."),
        1 => Ok(()),
        2 => conditions_ready(&rule.conditions),
        _ => action_ready(&rule.actions),
    }
}

fn conditions_ready(conditions: &[Condition]) -> Result<(), &'static str> {
    if conditions.is_empty() {
        return Err("Ajoutez au moins une condition.");
    }
    for c in conditions {
        let value = c.value.trim();
        if value.is_empty() {
            return Err("Complétez chaque condition.");
        }
        let understood = match (c.field, c.op) {
            (Field::Size, _) => parse_size(value).is_some(),
            (Field::DateAdded | Field::DateModified, Op::WithinLast | Op::NotWithinLast) => parse_span(value).is_some(),
            _ => true,
        };
        if !understood {
            return Err("Une valeur n'est pas comprise : vérifiez-la.");
        }
    }
    Ok(())
}

fn action_ready(actions: &[Action]) -> Result<(), &'static str> {
    let [action] = actions else { return Err("Choisissez ce qu'il faut faire des fichiers.") };
    let missing = |key: &str| action.param(key).is_empty();
    match action.kind {
        ActionKind::Move | ActionKind::Copy if missing("destination") => Err("Choisissez le dossier de destination."),
        ActionKind::Rename if missing("pattern") => Err("Indiquez le nouveau nom."),
        ActionKind::AddTag if missing("tags") => Err("Indiquez l'étiquette à ajouter."),
        _ => Ok(()),
    }
}

/// « 30 days » → (« 30 », « jours ») ; « 100 MB » → (« 100 », « MB »). Sert aux champs « nombre + unité ».
pub fn split_amount(value: &str) -> (String, String) {
    let t = value.trim();
    let end = t.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(t.len());
    (t[..end].trim().to_string(), t[end..].trim().to_string())
}

fn unit_fr(unit: &str, plural: bool) -> String {
    let base = match unit.to_lowercase().trim_end_matches('s') {
        "day" | "" => "jour",
        "week" => "semaine",
        "month" => "mois",
        "year" => return if plural { "ans".into() } else { "an".into() },
        _ => return unit.to_string(),
    };
    if plural && base != "mois" {
        format!("{base}s")
    } else {
        base.to_string()
    }
}

fn span_fr(value: &str) -> String {
    let (n, unit) = split_amount(value);
    let plural = n.replace(',', ".").parse::<f64>().map_or(true, |x| x > 1.0);
    format!("{n} {}", unit_fr(&unit, plural))
}

fn kind_phrase(value: &str, negative: bool) -> String {
    let noun = match value.trim().to_lowercase().as_str() {
        "image" => "une image",
        "document" => "un document",
        "video" => "une vidéo",
        "audio" => "un fichier audio",
        "archive" => "une archive",
        "folder" => "un dossier",
        "app" => "une application",
        "other" => "d'un autre type",
        other => return format!("{} {other}", if negative { "n'est pas du type" } else { "est du type" }),
    };
    format!("{} {noun}", if negative { "n'est pas" } else { "est" })
}

/// Une condition en français, au début de « quand un fichier … ».
pub fn condition_phrase(c: &Condition) -> String {
    let v = c.value.trim();
    let shown = if v.is_empty() { "…" } else { v };
    match (c.field, c.op) {
        (Field::Kind, Op::Is) => kind_phrase(v, false),
        (Field::Kind, Op::IsNot) => kind_phrase(v, true),
        (Field::Extension, Op::IsNot) => format!("n'a pas l'extension {shown}"),
        (Field::Extension, _) if shown.contains(',') => format!("a l'une des extensions {shown}"),
        (Field::Extension, _) => format!("a l'extension {shown}"),
        (Field::Name, Op::Contains) => format!("a un nom qui contient « {shown} »"),
        (Field::Name, Op::StartsWith) => format!("a un nom qui commence par « {shown} »"),
        (Field::Name, Op::EndsWith) => format!("a un nom qui finit par « {shown} »"),
        (Field::Name, _) => format!("s'appelle « {shown} »"),
        (Field::Size, Op::Greater) => format!("pèse plus de {shown}"),
        (Field::Size, Op::Less) => format!("pèse moins de {shown}"),
        (Field::Size, _) => format!("pèse exactement {shown}"),
        (Field::DateAdded | Field::DateModified, Op::WithinLast | Op::NotWithinLast) => {
            let verb = if c.field == Field::DateAdded { "a été ajouté" } else { "a été modifié" };
            if c.op == Op::NotWithinLast {
                format!("{verb} il y a plus de {}", span_fr(shown))
            } else {
                format!("{verb} dans les {} derniers", span_fr(shown))
            }
        }
        (Field::DateAdded | Field::DateModified, op) => {
            let what = if c.field == Field::DateAdded { "a été ajouté" } else { "a été modifié" };
            format!("{what} {} {shown}", if op == Op::Before { "avant le" } else { "après le" })
        }
        (Field::Tag, _) => format!("porte l'étiquette « {shown} »"),
        (Field::Contents, _) => format!("contient le texte « {shown} »"),
        (field, op) => format!("{} {} {shown}", field.label().to_lowercase(), op.label()),
    }
}

/// Une action en français, après « … , ».
pub fn action_phrase(a: &Action) -> String {
    let quoted = |key: &str| {
        let v = a.param(key);
        if v.is_empty() {
            "…".to_string()
        } else {
            format!("« {v} »")
        }
    };
    match a.kind {
        ActionKind::Move => format!("le déplacer vers {}", quoted("destination")),
        ActionKind::Copy => format!("le copier vers {}", quoted("destination")),
        ActionKind::Rename => format!("le renommer avec le motif {}", quoted("pattern")),
        ActionKind::Trash => "le mettre à la corbeille".into(),
        ActionKind::AddTag => format!("lui ajouter l'étiquette {}", quoted("tags")),
        ActionKind::Compress => "le compresser en archive ZIP".into(),
        _ => a.kind.label().to_lowercase(),
    }
}

/// La règle en une phrase : « Dans Téléchargements, quand un fichier est une image, le déplacer vers « Images ». »
pub fn rule_sentence(rule: &Rule, folders: &[Folder]) -> String {
    let names: Vec<String> = rule.folder_ids.iter().map(|id| tags::folder_label(folders, *id)).filter(|n| !n.is_empty()).collect();
    let place = match names.as_slice() {
        [] => "…".to_string(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} et {last}", init.join(", ")),
    };
    let when = if rule.conditions.is_empty() {
        "…".to_string()
    } else {
        rule.conditions.iter().map(condition_phrase).collect::<Vec<_>>().join(" et ")
    };
    let what = rule.actions.first().map_or_else(|| "…".to_string(), action_phrase);
    let prefix = if names.len() > 1 { "Dans les dossiers" } else { "Dans" };
    format!("{prefix} {place}, quand un fichier {when}, {what}.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(id: u64, path: &str) -> Folder {
        Folder { id, path: PathBuf::from(path), enabled: true, priority: 0 }
    }

    fn rule(conditions: Vec<Condition>, actions: Vec<Action>) -> Rule {
        Rule {
            id: 1,
            folder_ids: vec![1],
            name: "r".into(),
            enabled: true,
            match_all: true,
            recursion_depth: 0,
            priority: 0,
            conditions,
            actions,
        }
    }

    fn move_to(dest: &str) -> Action {
        let mut a = Action::new(ActionKind::Move);
        a.params.insert("destination".into(), dest.into());
        a
    }

    #[test]
    fn a_plain_rule_fits_the_simple_mode() {
        assert!(is_simple_compatible(&rule(vec![(PRESETS[0].make)()], vec![move_to("/x")])));
        assert!(is_simple_compatible(&rule(vec![], vec![])), "une règle vide se remplit dans l'assistant");
    }

    #[test]
    fn advanced_settings_keep_a_rule_out_of_the_simple_mode() {
        let base = || rule(vec![(PRESETS[0].make)()], vec![move_to("/x")]);
        let mut r = base();
        r.recursion_depth = 1;
        assert!(!is_simple_compatible(&r), "récursion");
        let mut r = base();
        r.conditions.push((PRESETS[1].make)());
        r.match_all = false;
        assert!(!is_simple_compatible(&r), "au moins une condition parmi plusieurs");
        let mut r = base();
        r.match_all = false;
        assert!(is_simple_compatible(&r), "avec une seule condition, le choix n'a pas d'effet");
        let mut r = base();
        r.conditions[0] = cond(Field::Name, Op::Matches, "^a");
        assert!(!is_simple_compatible(&r), "expression régulière");
        let mut r = base();
        r.actions.push(Action::new(ActionKind::Trash));
        assert!(!is_simple_compatible(&r), "plusieurs actions");
        let mut r = base();
        r.actions = vec![Action::new(ActionKind::Delete)];
        assert!(!is_simple_compatible(&r), "suppression définitive");
    }

    #[test]
    fn simple_actions_never_include_the_irreversible_ones() {
        assert!(!SIMPLE_ACTIONS.contains(&ActionKind::Delete));
        assert!(SIMPLE_ACTIONS.contains(&ActionKind::Trash));
    }

    #[test]
    fn simple_operators_drop_regular_expressions() {
        assert!(!simple_ops(Field::Name).contains(&Op::Matches));
        assert!(simple_ops(Field::Name).contains(&Op::Contains));
        assert!(!simple_ops(Field::Contents).contains(&Op::Matches));
    }

    #[test]
    fn every_preset_builds_a_condition_the_engine_understands() {
        for p in PRESETS {
            let c = (p.make)();
            assert!(c.field.ops().contains(&c.op), "{} : opérateur invalide", p.label);
            assert_ne!(c.op, Op::Matches);
        }
    }

    #[test]
    fn steps_say_what_is_missing() {
        let mut r = rule(vec![], vec![]);
        assert!(step_ready(1, &r).is_ok());
        r.folder_ids.clear();
        assert_eq!(step_ready(1, &r), Err("Choisissez au moins un dossier."));
        assert_eq!(step_ready(2, &r), Err("Ajoutez au moins une condition."));
        r.conditions.push(cond(Field::Name, Op::Contains, "  "));
        assert_eq!(step_ready(2, &r), Err("Complétez chaque condition."));
        r.conditions[0].value = "facture".into();
        assert!(step_ready(2, &r).is_ok());
        r.conditions.push(cond(Field::Size, Op::Greater, "beaucoup"));
        assert_eq!(step_ready(2, &r), Err("Une valeur n'est pas comprise : vérifiez-la."));
        r.conditions[1].value = "50 MB".into();
        assert!(step_ready(2, &r).is_ok());
        assert_eq!(step_ready(3, &r), Err("Choisissez ce qu'il faut faire des fichiers."));
        r.actions = vec![Action::new(ActionKind::Move)];
        assert_eq!(step_ready(3, &r), Err("Choisissez le dossier de destination."));
        r.actions = vec![move_to("/Images")];
        assert!(step_ready(3, &r).is_ok());
        r.actions = vec![Action::new(ActionKind::Trash)];
        assert!(step_ready(3, &r).is_ok(), "la corbeille n'a pas de réglage");
    }

    #[test]
    fn amounts_split_into_number_and_unit() {
        assert_eq!(split_amount("30 days"), ("30".into(), "days".into()));
        assert_eq!(split_amount("100MB"), ("100".into(), "MB".into()));
        assert_eq!(split_amount("1.5 gb"), ("1.5".into(), "gb".into()));
        assert_eq!(split_amount(""), (String::new(), String::new()));
    }

    #[test]
    fn the_sentence_reads_like_french() {
        let folders = vec![folder(1, "/Users/a/Téléchargements"), folder(2, "/Users/a/Bureau")];
        let r = rule(vec![(PRESETS[0].make)()], vec![move_to("/Users/a/Images")]);
        assert_eq!(
            rule_sentence(&r, &folders),
            "Dans Téléchargements, quand un fichier est une image, le déplacer vers « /Users/a/Images »."
        );
        let mut r = rule(vec![(PRESETS[4].make)(), (PRESETS[6].make)()], vec![Action::new(ActionKind::Trash)]);
        r.folder_ids = vec![1, 2];
        assert_eq!(
            rule_sentence(&r, &folders),
            "Dans les dossiers Téléchargements et Bureau, quand un fichier a été modifié il y a plus de 30 jours et pèse plus de 100 MB, le mettre à la corbeille."
        );
    }

    #[test]
    fn an_unfinished_rule_shows_dots_instead_of_panicking() {
        let r = rule(vec![], vec![]);
        assert_eq!(rule_sentence(&r, &[folder(1, "/a/Docs")]), "Dans Docs, quand un fichier …, ….");
    }

    #[test]
    fn spans_are_translated() {
        assert_eq!(span_fr("1 week"), "1 semaine");
        assert_eq!(span_fr("2 weeks"), "2 semaines");
        assert_eq!(span_fr("3 months"), "3 mois");
        assert_eq!(span_fr("1 year"), "1 an");
    }
}
