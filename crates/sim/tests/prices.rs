//! Unit prices (prompt 045 and its decision 4): range and rate of fire
//! enter the ammunition price, and a stronger unit is never cheaper.
//! `just economy-table` prints every price.

use sim::Catalog;
use sim::combat::CombatType;
use sim::economy::EconomyRules;
use sim::scale::Scale;

fn price(id: &str) -> [u32; 3] {
    let rules = EconomyRules::builtin().unwrap();
    let catalog = Catalog::builtin().unwrap();
    let scale = Scale::builtin().unwrap();
    let e = catalog.get(id).unwrap();
    rules
        .unit_cost(e, &CombatType::from_catalog(e, &scale))
        .resources
}

fn total(id: &str) -> u32 {
    price(id).iter().sum()
}

#[test]
fn rate_of_fire_enters_the_price() {
    // Same range (250 m), F-34 76.2 mm at 5 rounds/min against the M3
    // 75 mm at 10: the T-34 pays less ammunition although its gun is larger.
    assert!(price("su_t34_76_1943")[1] < price("us_m4a3_75")[1]);
    // Battle mode gives equal strength with 8 Panzer IV, 10 T-34 and 9
    // M4A3 (docs/BALANCE.md); at these prices the armies cost within 5 %.
    // A change of weapon ranges (prompt 067) has to measure this again.
    let army = |id: &str, n: u32| total(id) * n;
    let pz4 = army("de_pz4_h", 8);
    for other in [army("su_t34_76_1943", 10), army("us_m4a3_75", 9)] {
        assert!(
            other.abs_diff(pz4) * 100 <= pz4 * 5,
            "{other} against {pz4}"
        );
    }
    // A stronger tank is still never cheaper in any resource.
    for i in 0..3 {
        assert!(price("de_tiger1_e")[i] >= price("de_pz4_h")[i]);
    }
}
