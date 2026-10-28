use crate::event::MapState;
use crate::world::PlayerSnapshot;

macro_rules! queries {
    ($($variant:ident { $($field:ident: $input:ty),* } => $output:ty),* $(,)?) => {
        #[derive(Debug, serde::Deserialize, ts_rs::TS)]
        #[serde(tag = "kind", rename_all = "camelCase")]
        pub enum Query {
            $($variant { $($field: $input),* }),*
        }

        paste::paste! {
            pub trait Answer {
                $(fn [<$variant:snake>](&self, $($field: $input),*) -> $output;)*
            }

            pub fn answer(state: &impl Answer, query: Query) -> String {
                match query {
                    $(Query::$variant { $($field),* } =>
                        serde_json::to_string(&state.[<$variant:snake>]($($field),*)).unwrap()),*
                }
            }
        }

        pub fn query_outputs(config: &ts_rs::Config) -> String {
            let mut declaration = String::from("export type QueryOutputs = { ");
            $(
                let mut key = stringify!($variant).to_owned();
                key[..1].make_ascii_lowercase();
                declaration.push_str(&format!("{}: {}, ", key, <$output as ts_rs::TS>::name(config)));
            )*
            declaration.push_str("};\n");
            declaration
        }
    };
}

queries! {
    Map {} => MapState,
    Me {} => Option<PlayerSnapshot>,
}
