use crate::simple_bt::{BehaviorArc, BehaviorNode, NodeResult};
use std::fmt::Debug;
use std::sync::Arc;

/// Repeats its child infintely
pub struct Repeated<B> {
    resume: Option<BehaviorArc<B>>,
    child: BehaviorArc<B>,
}

impl<B> Debug for Repeated<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Repeated")
            .field("child", &self.child)
            .finish()
    }
}

impl<B> Repeated<B> {
    pub fn new(child: BehaviorArc<B>) -> Self {
        Self {
            child,
            resume: None,
        }
    }
}

impl<B: 'static> BehaviorNode<B> for Repeated<B> {
    fn tick(&self, blackboard: &mut B) -> NodeResult<B> {
        if let Some(resume) = self.resume.as_ref() {
            match resume.tick(blackboard) {
                NodeResult::Running(resume) => {
                    return NodeResult::Running(
                        Self {
                            resume: Some(resume),
                            child: self.child.clone(),
                        }
                        .arc(),
                    )
                }
                _ => {}
            }
        }
        match self.child.tick(blackboard) {
            NodeResult::Running(resume) => {
                return NodeResult::Running(
                    Self {
                        resume: Some(resume),
                        child: self.child.clone(),
                    }
                    .arc(),
                )
            }
            _ => {}
        }

        // Restart, cuz we never end
        NodeResult::Running(Arc::new(Self {
            child: self.child.clone(),
            resume: None,
        }))
    }
}

/// Repeats its child a set number of times
pub struct LimitedRepeated<B> {
    child: BehaviorArc<B>,
    limit: usize,
    completed: usize,
}

impl<B> Debug for LimitedRepeated<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<B> BehaviorNode<B> for LimitedRepeated<B> {
    fn tick(&self, blackboard: &mut B) -> NodeResult<B> {
        todo!()
    }
}

/// Repeats its child until its child fails
pub struct RepeatedUntilFailure<B> {
    child: BehaviorArc<B>,
}

impl<B> Debug for RepeatedUntilFailure<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepeatedUntilFailure")
            .field("child", &self.child)
            .finish()
    }
}

impl<B> BehaviorNode<B> for RepeatedUntilFailure<B> {
    fn tick(&self, blackboard: &mut B) -> NodeResult<B> {
        todo!()
    }
}

#[cfg(test)]
mod tests {}
