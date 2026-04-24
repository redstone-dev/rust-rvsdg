use crate::{Context, Origin, User, id, node_kind_impl};
use std::any::Any;

pub trait NodeKind: std::any::Any + std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn node_type(&self) -> &str;
}

/// Only set temporarily before we've initialised a node
#[derive(Debug, Clone)]
pub struct Uninitialized;
node_kind_impl!(Uninitialized, "uninitialized");

#[derive(Debug, Clone)]
pub struct Apply {}
node_kind_impl!(Apply, "apply");

#[derive(Debug, Clone)]
pub struct DoWhile {}
node_kind_impl!(DoWhile, "theta");

#[derive(Debug, Clone)]
pub struct GlobalV {}
node_kind_impl!(GlobalV, "delta");

#[derive(Debug, Clone)]
pub struct Lambda {}
node_kind_impl!(Lambda, "lambda");

#[derive(Debug, Clone)]
pub struct TranslationUnit {}
node_kind_impl!(TranslationUnit, "omega");

#[derive(Debug, Clone)]
pub struct Number(pub i128);
node_kind_impl!(Number, "number");

#[derive(Debug, Clone)]
pub struct Switch;
node_kind_impl!(Switch, "gamma");

#[derive(Debug, Clone)]
pub struct Placeholder(pub &'static str);
node_kind_impl!(Placeholder, "placeholder");

#[derive(Debug, Clone)]
pub struct RecEnv {}
node_kind_impl!(RecEnv, "phi");

#[macro_export]
macro_rules! node_kind_impl {
    ($ty:ty, $kind:literal) => {
        impl NodeKind for $ty {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn node_type(&self) -> &str {
                $kind
            }
        }
    };
}

impl Context {
    pub fn move_to_new_recenv(&mut self, [origin_lambda, user_lambda]: [id::Node<Lambda>; 2]) {
        let env = self.add_recenv_node();
        let env_region = self.only_child_region(env.id);

        // Disconnect all connections to these these lambdas
        let mut disconnected: Vec<(Origin, User)> = vec![];
        self.for_each_edge(origin_lambda.id, |origin, user| {
            disconnected.push((origin, user))
        });
        self.for_each_edge(user_lambda.id, |origin, user| {
            disconnected.push((origin, user))
        });

        self.move_node(origin_lambda.id, env_region);
        self.move_node(user_lambda.id, env_region);

        // Re-create all the connections we disconnected
        for (origin, user) in disconnected {
            self.connect(origin, user);
        }
    }
}
