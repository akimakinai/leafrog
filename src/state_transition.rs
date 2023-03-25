use bevy::prelude::*;
use std::marker::PhantomData;

pub struct StateTransitionEvent<S>(PhantomData<S>);

impl<S> Default for StateTransitionEvent<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

fn send_transition_event<S: States>(
    mut tx: EventWriter<StateTransitionEvent<S>>,
) {
    tx.send_default();
}

pub struct StateTransitionDetectorPlugin<S>(PhantomData<S>);

impl<S> Default for StateTransitionDetectorPlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

// #[derive(StageLabel, Clone, Hash, Debug, Eq, PartialEq, Ord, PartialOrd)]
// struct StateTransitionDetectorStage;

impl<S: States> Plugin for StateTransitionDetectorPlugin<S> {
    fn build(&self, app: &mut App) {
        // let stage = SystemStage::parallel().with_system(state_transition_detector::<S>);

        // // detector must run before NextState is removed in StateTransitionStage.
        // app.add_event::<StateTransitionEvent<S>>().add_stage_before(
        //     iyes_loopless::state::StateTransitionStageLabel::from_type::<S>(),
        //     StateTransitionDetectorStage,
        //     stage,
        // );
        app.add_event::<StateTransitionEvent<S>>();

        for v in S::variants() {
            app.add_system(send_transition_event::<S>.in_schedule(OnEnter(v)));
        }
    }
}
