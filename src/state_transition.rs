use bevy::ecs::schedule::StateData;
use bevy::prelude::*;
use iyes_loopless::prelude::*;
use std::marker::PhantomData;

pub struct StateTransitionEvent<S>(PhantomData<S>);

impl<S> Default for StateTransitionEvent<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

fn state_transition_detector<S: StateData>(
    next_state: Option<Res<NextState<S>>>,
    mut tx: EventWriter<StateTransitionEvent<S>>,
) {
    if next_state.is_some() {
        tx.send_default();
    }
}

pub struct StateTransitionDetectorPlugin<S>(PhantomData<S>);

impl<S> Default for StateTransitionDetectorPlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(StageLabel, Clone, Hash, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct StateTransitionDetectorStage;

impl<S: StateData> Plugin for StateTransitionDetectorPlugin<S> {
    fn build(&self, app: &mut App) {
        let stage = SystemStage::parallel().with_system(state_transition_detector::<S>);

        // detector must run before NextState is removed in StateTransitionStage.
        app.add_event::<StateTransitionEvent<S>>().add_stage_before(
            iyes_loopless::state::StateTransitionStageLabel::from_type::<S>(),
            StateTransitionDetectorStage,
            stage,
        );
    }
}
