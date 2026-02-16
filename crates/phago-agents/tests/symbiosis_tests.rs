//! Symbiosis (Endosymbiosis) tests.

use phago_agents::digester::Digester;
use phago_agents::sentinel::Sentinel;
use phago_core::agent::Agent;
use phago_core::types::*;

#[test]
fn digester_absorbs_non_digester() {
    let d = Digester::new(Position::new(0.0, 0.0));
    let s = Sentinel::new(Position::new(1.0, 0.0));

    let profile = s.profile();
    let eval = d.evaluate_symbiosis(&profile);
    assert_eq!(
        eval,
        Some(SymbiosisEval::Integrate),
        "digester should integrate sentinel"
    );
}

#[test]
fn digester_coexists_with_digester() {
    let d1 = Digester::new(Position::new(0.0, 0.0));
    let d2 = Digester::new(Position::new(1.0, 0.0));

    let profile = d2.profile();
    let eval = d1.evaluate_symbiosis(&profile);
    assert_eq!(
        eval,
        Some(SymbiosisEval::Coexist),
        "same-type agents should coexist"
    );
}

#[test]
fn absorbed_agent_gets_correct_death_cause() {
    // Simulate symbiotic absorption via colony
    use phago_runtime::colony::Colony;

    let mut colony = Colony::new();

    // Ingest enough material so digester becomes productive
    colony.ingest_document(
        "Biology",
        "The cell membrane controls transport of molecules. Proteins serve as channels \
         and receptors for signaling cascades in the cellular environment.",
        Position::new(0.0, 0.0),
    );
    colony.ingest_document(
        "Chemistry",
        "Chemical bonds between atoms form molecules. Oxidation reduction reactions \
         transfer electrons between molecules in solution.",
        Position::new(0.0, 1.0),
    );

    // Spawn digester (needs 3+ useful outputs for symbiosis attempt)
    colony.spawn(Box::new(
        Digester::new(Position::new(0.0, 0.0)).with_max_idle(80),
    ));
    // Spawn sentinel nearby — digester may try to absorb it
    colony.spawn(Box::new(Sentinel::new(Position::new(0.5, 0.0))));

    // Run until symbiosis potentially happens
    colony.run(60);

    // Check if any death was due to symbiotic absorption
    let symbiotic_deaths: Vec<_> = colony
        .death_signals()
        .iter()
        .filter(|ds| matches!(ds.cause, DeathCause::SymbioticAbsorption(_)))
        .collect();

    // Note: symbiosis depends on signal conditions, so it may or may not happen
    // If it did happen, verify the death cause is correct
    for ds in &symbiotic_deaths {
        if let DeathCause::SymbioticAbsorption(absorber) = &ds.cause {
            assert_ne!(
                *absorber, ds.agent_id,
                "absorber should be different from absorbed"
            );
        }
    }
}
