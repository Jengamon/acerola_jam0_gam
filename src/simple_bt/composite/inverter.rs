use crate::simple_bt::{BehaviorArc, BehaviorNode, NodeResult};

/// Inverts the result of its child
///
/// Success becomes failure, failure success.
pub struct Inverter<B> {
    child: BehaviorArc<B>,
}

impl<B> std::fmt::Debug for Inverter<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inverter")
            .field("child", &self.child)
            .finish()
    }
}

impl<B> Inverter<B> {
    pub fn new(child: BehaviorArc<B>) -> Self {
        Self { child }
    }
}

impl<B: 'static> BehaviorNode<B> for Inverter<B> {
    fn tick(&self, blackboard: &mut B) -> NodeResult<B> {
        match self.child.tick(blackboard) {
            NodeResult::Success => NodeResult::Failure,
            NodeResult::Failure => NodeResult::Success,
            NodeResult::Running(resume) => NodeResult::Running(Inverter::new(resume).arc()),
        }
    }
}

#[cfg(test)]
mod tests {}
