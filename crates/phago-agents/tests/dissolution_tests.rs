//! Dissolution (Holobiont boundary modulation) tests.

use phago_agents::digester::Digester;
use phago_core::agent::Agent;
use phago_core::types::*;

#[test]
fn permeability_zero_initially() {
    let d = Digester::new(Position::new(0.0, 0.0));
    assert_eq!(
        d.permeability(),
        0.0,
        "fresh digester should have zero permeability"
    );
}

#[test]
fn permeability_increases_with_context() {
    let mut d = Digester::new(Position::new(0.0, 0.0));
    // Digest something so it has vocabulary
    d.digest_text("cell membrane protein transport molecular biology".to_string());

    // High reinforcement, high age, high trust
    let context = BoundaryContext {
        reinforcement_count: 50,
        age: 200,
        trust: 0.9,
    };
    d.modulate_boundary(&context);

    assert!(
        d.permeability() > 0.5,
        "high context should produce permeability > 0.5, got {}",
        d.permeability()
    );
}

#[test]
fn externalize_includes_all_vocab() {
    let mut d = Digester::new(Position::new(0.0, 0.0));

    // Own digestion
    d.digest_text("cell membrane protein transport".to_string());

    // Integrate foreign vocabulary
    let mut producer = Digester::new(Position::new(5.0, 0.0));
    producer.digest_text("mitochondria energy oxidative phosphorylation".to_string());
    let vocab_bytes = producer.export_vocabulary().unwrap();
    d.integrate_vocabulary(&vocab_bytes);

    let externalized = d.externalize_vocabulary();

    // Should contain own presentations
    assert!(
        externalized.contains(&"cell".to_string()),
        "should contain own term 'cell'"
    );
    // Should contain integrated terms
    assert!(
        externalized.contains(&"mitochondria".to_string()),
        "should contain integrated term 'mitochondria'"
    );
}
