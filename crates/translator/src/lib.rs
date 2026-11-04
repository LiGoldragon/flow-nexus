//! Text edge for the Flow Nexus public signal.

use datom_codec::{Actualizing, Budget, Potential};
use signal_flow_nexus_version_1::{Query, Response};

pub fn decode(text: &str, budget: &mut Budget) -> Result<Query, String> {
    Potential::<Query>::from(text).actualize(budget).map_err(|error| format!("{error:?}"))
}

pub fn encode(response: &Response) -> String { translator::encode(response) }

#[cfg(test)]
mod tests {
    use super::*;
    use protos::ReaderBudget;
    use signal_flow_nexus_version_1::{ComplexName, Query, Response};

    fn budget() -> Budget {
        Budget { remaining: 4096, reader: ReaderBudget { remaining: 4096 }, depth: 0, maximum_depth: 32 }
    }

    #[test]
    fn public_text_round_trip() {
        let query = decode("Describe.main", &mut budget()).unwrap();
        assert_eq!(query, Query::Describe(ComplexName("main".into())));
        assert_eq!(encode(&Response::Listed(vec![])), "Listed.[]");
    }
}
