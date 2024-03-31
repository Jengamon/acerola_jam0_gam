use crate::simple_bt::{BehaviorArc, BehaviorNode, NodeResult};
use std::sync::Arc;

pub struct Sequence<B> {
    pub(crate) sub: Arc<[BehaviorArc<B>]>,
}

impl<B> std::fmt::Debug for Sequence<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Sequence<{:p}>", self.sub.as_ref()))
            .field("sub", &self.sub)
            .finish()
    }
}

impl<B, I: Into<BehaviorArc<B>>> FromIterator<I> for Sequence<B> {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self {
            sub: Arc::from(iter.into_iter().map(Into::into).collect::<Vec<_>>()),
        }
    }
}

impl<B: 'static> Sequence<B> {
    pub(crate) fn resume(
        seq: Arc<[BehaviorArc<B>]>,
        index: usize,
        resume: BehaviorArc<B>,
    ) -> BehaviorArc<B> {
        SequenceResume { seq, resume, index }.arc()
    }
}

impl<B: 'static> BehaviorNode<B> for Sequence<B> {
    fn tick(&self, blackboard: &mut B) -> NodeResult<B> {
        for (idx, sub) in self.sub.iter().enumerate() {
            match sub.tick(blackboard) {
                NodeResult::Success => {}
                NodeResult::Failure => return NodeResult::Failure,
                NodeResult::Running(resume) => {
                    return NodeResult::Running(Self::resume(self.sub.clone(), idx, resume))
                }
            }
        }
        NodeResult::Success
    }
}

pub(crate) struct SequenceResume<B> {
    pub(crate) seq: Arc<[BehaviorArc<B>]>,
    pub(crate) resume: BehaviorArc<B>,
    pub(crate) index: usize,
}

impl<B> std::fmt::Debug for SequenceResume<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("SequenceResume<{:p}>", self.seq.as_ref()))
            .field("resume", &self.resume)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl<B: 'static> BehaviorNode<B> for SequenceResume<B> {
    fn tick(&self, blackboard: &mut B) -> NodeResult<B> {
        // Tick the node we want to resume on
        match self.resume.tick(blackboard) {
            NodeResult::Success => {}
            NodeResult::Failure => return NodeResult::Failure,
            NodeResult::Running(resume) => {
                return NodeResult::Running(Sequence::resume(self.seq.clone(), self.index, resume))
            }
        }
        for (idx, sub) in self.seq.iter().enumerate().skip(self.index) {
            match sub.tick(blackboard) {
                NodeResult::Success => {}
                NodeResult::Failure => return NodeResult::Failure,
                NodeResult::Running(resume) => {
                    return NodeResult::Running(Sequence::resume(self.seq.clone(), idx, resume))
                }
            }
        }
        NodeResult::Success
    }
}

#[cfg(test)]
mod tests {
    use super::{BehaviorNode, NodeResult, Sequence};
    use crate::simple_bt::BehaviorRunner;
    use assert2::check;
    use bevy::prelude::*;

    #[derive(Debug)]
    struct MoveTo {
        part: f32,
        goal: Vec2,
    }

    impl BehaviorNode<Vec2> for MoveTo {
        fn tick(&self, position: &mut Vec2) -> NodeResult<Vec2> {
            let movement = (self.goal - *position) * self.part.recip();
            *position += movement;

            const ERROR: f32 = 0.001;
            if (self.goal - *position).length() < ERROR {
                NodeResult::Success
            } else {
                NodeResult::Running(
                    Self {
                        part: self.part,
                        goal: self.goal,
                    }
                    .arc(),
                )
            }
        }
    }

    #[test]
    fn test_seequence() {
        let tree = [
            MoveTo {
                part: 5.0,
                goal: Vec2::ZERO,
            }
            .arc(),
            MoveTo {
                part: 5.0,
                goal: Vec2::splat(5.0),
            }
            .arc(),
        ]
        .into_iter()
        .collect::<Sequence<_>>()
        .arc();

        // Set some position
        let mut position = Vec2::X * 5.0;

        // If a position is set, we will execute the action
        {
            let mut runner = BehaviorRunner::new(tree.clone());
            let mut res = None;
            while res.is_none() {
                res = runner.proceed(&mut position);
            }
            check!(res.is_some_and(|v| v));
        }
    }
}
