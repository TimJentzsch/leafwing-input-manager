//! This module contains [`ActionState`] and its supporting methods and impls.

use crate::Actionlike;
use bevy::prelude::*;
use bevy::utils::{Duration, Instant};
use bevy::utils::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

/// The current state of a particular virtual button,
/// corresponding to a single [`Actionlike`] action.
///
/// Detailed timing information for the button can be accessed through the stored [`Timing`] value
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum VirtualButtonState {
    /// This button is currently pressed
    Pressed(Timing),
    /// This button is currently released
    Released(Timing),
}

/// Stores the timing information for a [`VirtualButtonState`]
#[derive(Debug, Clone, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct Timing {
    /// The [`Instant`] at which the button was pressed or released
    ///
    /// Recorded as the [`Time`](bevy::core::Time) at the start of the tick after the state last changed.
    /// If this is none, [`ActionState::update`] has not been called yet.
    #[serde(skip)]
    pub instant_started: Option<Instant>,
    /// The [`Duration`] for which the button has been pressed or released.
    ///
    /// This begins at [`Duration::ZERO`] when [`ActionState::update`] is called.
    pub current_duration: Duration,
    /// The [`Duration`] for which the button was pressed or released before the state last changed.
    pub previous_duration: Duration,
}

impl VirtualButtonState {
    /// Is the button currently pressed?
    #[inline]
    #[must_use]
    pub fn pressed(&self) -> bool {
        match self {
            VirtualButtonState::Pressed(_) => true,
            VirtualButtonState::Released(_) => false,
        }
    }

    /// Is the button currently released?
    #[inline]
    #[must_use]
    pub fn released(&self) -> bool {
        match self {
            VirtualButtonState::Pressed(_) => false,
            VirtualButtonState::Released(_) => true,
        }
    }

    /// Was the button pressed since the last time [`ActionState::update`] was called?
    #[inline]
    #[must_use]
    pub fn just_pressed(&self) -> bool {
        match self {
            VirtualButtonState::Pressed(timing) => timing.instant_started.is_none(),
            VirtualButtonState::Released(_timing) => false,
        }
    }

    /// Was the button released since the last time [`ActionState::update`] was called?
    #[inline]
    #[must_use]
    pub fn just_released(&self) -> bool {
        match self {
            VirtualButtonState::Pressed(_timing) => false,
            VirtualButtonState::Released(timing) => timing.instant_started.is_none(),
        }
    }

    /// The [`Instant`] at which the button was pressed or released
    ///
    /// Recorded as the [`Time`](bevy::core::Time) at the start of the tick after the state last changed.
    /// If this is none, [`ActionState::update`] has not been called yet.
    #[inline]
    #[must_use]
    pub fn instant_started(&self) -> Option<Instant> {
        match self {
            VirtualButtonState::Pressed(timing) => timing.instant_started,
            VirtualButtonState::Released(timing) => timing.instant_started,
        }
    }

    /// The [`Duration`] for which the button has been pressed or released.
    ///
    /// This begins at [`Duration::ZERO`] when [`ActionState::update`] is called.
    #[inline]
    #[must_use]
    pub fn current_duration(&self) -> Duration {
        match self {
            VirtualButtonState::Pressed(timing) => timing.current_duration,
            VirtualButtonState::Released(timing) => timing.current_duration,
        }
    }
    /// The [`Duration`] for which the button was pressed or released before the state last changed.
    #[inline]
    #[must_use]
    pub fn previous_duration(&self) -> Duration {
        match self {
            VirtualButtonState::Pressed(timing) => timing.previous_duration,
            VirtualButtonState::Released(timing) => timing.previous_duration,
        }
    }
}

impl Default for VirtualButtonState {
    fn default() -> Self {
        VirtualButtonState::Released(Timing::default())
    }
}

/// Stores the canonical input-method-agnostic representation of the inputs received
///
/// Intended to be used as a [`Component`] on entities that you wish to control directly from player input.
///
/// # Example
/// ```rust
/// use leafwing_input_manager::prelude::*;
/// use bevy::utils::Instant;
///
/// #[derive(Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug)]
/// enum Action {
///     Left,
///     Right,
///     Jump,
/// }
///
/// let mut action_state = ActionState::<Action>::default();
///
/// // Typically, this is done automatically by the `InputManagerPlugin` from user inputs
/// // using the `ActionState::update` method
/// action_state.press(Action::Jump);
///
/// assert!(action_state.pressed(Action::Jump));
/// assert!(action_state.just_pressed(Action::Jump));
/// assert!(action_state.released(Action::Left));
///
/// // Resets just_pressed and just_released
/// action_state.tick(Instant::now());
/// assert!(action_state.pressed(Action::Jump));
/// assert!(!action_state.just_pressed(Action::Jump));
///
/// action_state.release(Action::Jump);
/// assert!(!action_state.pressed(Action::Jump));
/// assert!(action_state.released(Action::Jump));
/// assert!(action_state.just_released(Action::Jump));
///
/// action_state.tick(Instant::now());
/// assert!(action_state.released(Action::Jump));
/// assert!(!action_state.just_released(Action::Jump));
/// ```
#[derive(Component, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionState<A: Actionlike> {
    map: HashMap<A, VirtualButtonState>,
}

impl<A: Actionlike> ActionState<A> {
    /// Updates the [`ActionState`] based on a [`HashSet`] of pressed virtual buttons.
    ///
    /// The `pressed_set` is typically constructed from [`InputMap::which_pressed`](crate::input_map::InputMap),
    /// which reads from the assorted [`Input`] resources.
    pub fn update(&mut self, pressed_set: HashSet<A>) {
        for action in A::iter() {
            match self.state(action.clone()) {
                VirtualButtonState::Pressed(_) => {
                    if !pressed_set.contains(&action) {
                        self.release(action);
                    }
                }
                VirtualButtonState::Released(_) => {
                    if pressed_set.contains(&action) {
                        self.press(action);
                    }
                }
            }
        }
    }

    /// Advances the time for all virtual buttons
    ///
    /// The underlying [`VirtualButtonState`] state will be advanced according to the `current_time`.
    /// - if no [`Instant`] is set, the `current_time` will be set as the initial time at which the button was pressed / released
    /// - the [`Duration`] will advance to reflect elapsed time
    ///
    /// # Example
    /// ```rust
    /// use leafwing_input_manager::prelude::*;
    /// use leafwing_input_manager::action_state::VirtualButtonState;
    /// use bevy::utils::Instant;
    ///
    /// #[derive(Actionlike, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    /// enum Action {
    ///     Run,
    ///     Jump,
    /// }
    ///
    /// let mut action_state = ActionState::<Action>::default();
    /// // Virtual buttons start released
    /// assert!(action_state.state(Action::Run).just_released());
    /// assert!(action_state.just_released(Action::Jump));
    ///
    /// // Ticking time moves causes buttons that were just released to no longer be just released
    /// action_state.tick(Instant::now());
    /// assert!(action_state.released(Action::Jump));
    /// assert!(!action_state.just_released(Action::Jump));
    ///
    /// action_state.press(Action::Jump);
    /// assert!(action_state.just_pressed(Action::Jump));
    ///
    /// // Ticking time moves causes buttons that were just pressed to no longer be just pressed
    /// action_state.tick(Instant::now());
    /// assert!(action_state.pressed(Action::Jump));
    /// assert!(!action_state.just_pressed(Action::Jump));
    /// ```
    pub fn tick(&mut self, current_instant: Instant) {
        use VirtualButtonState::*;

        for state in self.map.values_mut() {
            *state = match state {
                Pressed(timing) => match timing.instant_started {
                    Some(instant) => Pressed(Timing {
                        current_duration: current_instant - instant,
                        ..*timing
                    }),
                    None => Pressed(Timing {
                        instant_started: Some(current_instant),
                        current_duration: Duration::ZERO,
                        ..*timing
                    }),
                },
                Released(timing) => match timing.instant_started {
                    Some(instant) => Released(Timing {
                        current_duration: current_instant - instant,
                        ..*timing
                    }),
                    None => Released(Timing {
                        instant_started: Some(current_instant),
                        current_duration: Duration::ZERO,
                        ..*timing
                    }),
                },
            };
        }
    }

    /// Gets the [`VirtualButtonState`] of the corresponding `action`
    ///
    /// Generally, it'll be clearer to call `pressed` or so on directly on the [`ActionState`].
    /// However, accessing the state directly allows you to examine the detailed [`Timing`] information.
    ///
    /// # Example
    /// ```rust
    /// use leafwing_input_manager::prelude::*;
    /// use leafwing_input_manager::action_state::VirtualButtonState;
    ///
    /// #[derive(Actionlike, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    /// enum Action {
    ///     Run,
    ///     Jump,
    /// }
    /// let mut action_state = ActionState::<Action>::default();
    /// let run_state = action_state.state(Action::Run);
    /// // States can either be pressed or released,
    /// // and store an internal `Timing`
    /// if let VirtualButtonState::Pressed(timing) = run_state {
    ///     let pressed_duration = timing.current_duration;
    ///     let last_released_duration = timing.previous_duration;
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn state(&self, action: A) -> VirtualButtonState {
        if let Some(state) = self.map.get(&action) {
            state.clone()
        } else {
            VirtualButtonState::default()
        }
    }

    /// Manually sets the [`VirtualButtonState`] of the corresponding `action`
    ///
    /// You should almost always be using the [`ActionState::press`] and [`ActionState::release`] methods instead,
    /// as they will ensure that the duration is correct.
    ///
    /// However, this method can be useful for testing,
    /// or when transferring [`VirtualButtonState`] between action maps.
    ///
    /// # Example
    /// ```rust
    /// use leafwing_input_manager::prelude::*;
    /// use leafwing_input_manager::action_state::VirtualButtonState;
    ///
    /// #[derive(Actionlike, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    /// enum AbilitySlot {
    ///     Slot1,
    ///     Slot2,
    /// }
    ///
    /// #[derive(Actionlike, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    /// enum Action {
    ///     Run,
    ///     Jump,
    /// }
    ///
    /// let mut ability_slot_state = ActionState::<AbilitySlot>::default();
    /// let mut action_state = ActionState::<Action>::default();
    ///
    /// // Extract the state from the ability slot
    /// let slot_1_state = ability_slot_state.state(AbilitySlot::Slot1);
    ///
    /// // And transfer it to the actual ability that we care about
    /// // without losing timing information
    /// action_state.set_state(Action::Run, slot_1_state);
    /// ```
    #[inline]
    pub fn set_state(&mut self, action: A, state: VirtualButtonState) {
        let stored_state = self
            .map
            .get_mut(&action)
            .expect("Action {action} not found when setting state!");
        *stored_state = state;
    }

    /// Press the `action` virtual button
    pub fn press(&mut self, action: A) {
        if let VirtualButtonState::Released(timing) = self.state(action.clone()) {
            self.map.insert(
                action,
                VirtualButtonState::Pressed(Timing {
                    instant_started: None,
                    current_duration: Duration::ZERO,
                    previous_duration: timing.current_duration,
                }),
            );
        }
    }

    /// Release the `action` virtual button
    pub fn release(&mut self, action: A) {
        if let VirtualButtonState::Pressed(timing) = self.state(action.clone()) {
            self.map.insert(
                action,
                VirtualButtonState::Released(Timing {
                    instant_started: None,
                    current_duration: Duration::ZERO,
                    previous_duration: timing.current_duration,
                }),
            );
        }
    }

    /// Releases all action virtual buttons
    pub fn release_all(&mut self) {
        for action in A::iter() {
            self.release(action);
        }
    }

    /// Is this `action` currently pressed?
    #[inline]
    #[must_use]
    pub fn pressed(&self, action: A) -> bool {
        self.state(action).pressed()
    }

    /// Was this `action` pressed since the last time [tick](ActionState::tick) was called?
    #[inline]
    #[must_use]
    pub fn just_pressed(&self, action: A) -> bool {
        self.state(action).just_pressed()
    }

    /// Is this `action` currently released?
    ///
    /// This is always the logical negation of [pressed](ActionState::pressed)
    #[inline]
    #[must_use]
    pub fn released(&self, action: A) -> bool {
        self.state(action).released()
    }

    /// Was this `action` pressed since the last time [tick](ActionState::tick) was called?
    #[inline]
    #[must_use]
    pub fn just_released(&self, action: A) -> bool {
        self.state(action).just_released()
    }

    #[must_use]
    /// Which actions are currently pressed?
    pub fn get_pressed(&self) -> HashSet<A> {
        A::iter().filter(|a| self.pressed(a.clone())).collect()
    }

    #[must_use]
    /// Which actions were just pressed?
    pub fn get_just_pressed(&self) -> HashSet<A> {
        A::iter().filter(|a| self.just_pressed(a.clone())).collect()
    }

    #[must_use]
    /// Which actions are currently released?
    pub fn get_released(&self) -> HashSet<A> {
        A::iter().filter(|a| self.released(a.clone())).collect()
    }

    #[must_use]
    /// Which actions were just released?
    pub fn get_just_released(&self) -> HashSet<A> {
        A::iter()
            .filter(|a| self.just_released(a.clone()))
            .collect()
    }

    /// Creates a Hashmap with all of the possible A variants as keys, and false as the values
    #[inline]
    #[must_use]
    pub fn default_map<V: Default>() -> HashMap<A, V> {
        // PERF: optimize construction through pre-allocation or constification
        let mut map: HashMap<A, V> = HashMap::default();

        for action in A::iter() {
            map.insert(action, V::default());
        }
        map
    }
}

impl<A: Actionlike> Default for ActionState<A> {
    fn default() -> Self {
        Self {
            map: Self::default_map(),
        }
    }
}

/// A component that allows the attached entity to drive the [`ActionState`] of the associated entity
///
/// Used in [`update_action_state_from_interaction`](crate::systems::update_action_state_from_interaction).
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActionStateDriver<A: Actionlike> {
    /// The action triggered by this entity
    pub action: A,
    /// The entity whose action state should be updated
    pub entity: Entity,
}

/// Thresholds for when the `value` of a button will cause it to be pressed or released
///
/// Both `pressed` and `released` must be between 0.0 and 1.0 inclusive,
/// and `pressed` must be greater than `released`
/// Defaults to 0.5 for both values
#[derive(Debug, Clone)]
pub struct ButtonThresholds {
    pressed: f32,
    released: f32,
}

impl Default for ButtonThresholds {
    fn default() -> Self {
        Self {
            pressed: 0.5,
            released: 0.5,
        }
    }
}

impl ButtonThresholds {
    /// Gets the value at or above which the button is considered to be pressed
    #[inline]
    #[must_use]
    pub fn pressed(&self) -> f32 {
        self.pressed
    }

    /// Gets the value below which the button is considered to be released
    #[inline]
    #[must_use]
    pub fn released(&self) -> f32 {
        self.released
    }

    /// Sets the value of the pressed threshold.
    ///
    /// If the provided `value` is less than the `released` threshold,
    /// it is increased to the `released` threshold and a
    /// `ThresholdError(value_set_to)` error is returned.
    ///
    /// # Panics
    /// Panics if the value provided is not between 0.0 and 1.0 inclusive.
    pub fn set_pressed(&mut self, value: f32) -> Result<(), ThresholdError> {
        assert!(value >= 0.0);
        assert!(value <= 1.0);

        if value >= self.released {
            self.pressed = value;
            Ok(())
        } else {
            self.pressed = self.released;
            Err(ThresholdError(self.released))
        }
    }

    /// Gets the value below which the button is considered to be released
    ///
    /// If the provided `value` is greater than the `pressed` threshold,
    /// it is increased to the `pressed` threshold and a
    /// `ThresholdError(value_set_to)` error is returned.
    ///
    /// # Panics
    /// Panics if the value provided is not between 0.0 and 1.0 inclusive.
    pub fn set_released(&mut self, value: f32) -> Result<(), ThresholdError> {
        assert!(value >= 0.0);
        assert!(value <= 1.0);

        if value <= self.pressed {
            self.pressed = value;
            Ok(())
        } else {
            self.released = self.pressed;
            Err(ThresholdError(self.pressed))
        }
    }
}

/// An error that resulted from inserting an invalid (but within range value) to [`ButtonThresholds`]
#[derive(Debug, Clone)]
pub struct ThresholdError(f32);

mod tests {
    use crate as leafwing_input_manager;
    use crate::prelude::*;

    #[derive(Actionlike, Clone, Copy, PartialEq, Eq, Hash, Debug)]
    enum Action {
        Run,
        Jump,
        Hide,
    }

    #[test]
    fn press_lifecycle() {
        use crate::user_input::InputStreams;
        use bevy::prelude::*;
        use bevy::utils::Instant;

        // Action state
        let mut action_state = ActionState::<Action>::default();

        // Input map
        let mut input_map = InputMap::default();
        input_map.insert(Action::Run, KeyCode::R);

        // Input streams
        let mut keyboard_input_stream = Input::<KeyCode>::default();
        let input_streams = InputStreams::from_keyboard(&keyboard_input_stream);

        // Starting state
        action_state.update(input_map.which_pressed(&input_streams));

        assert!(!action_state.pressed(Action::Run));
        assert!(!action_state.just_pressed(Action::Run));
        assert!(action_state.released(Action::Run));
        assert!(action_state.just_released(Action::Run));

        // Pressing
        keyboard_input_stream.press(KeyCode::R);
        let input_streams = InputStreams::from_keyboard(&keyboard_input_stream);

        action_state.update(input_map.which_pressed(&input_streams));

        assert!(action_state.pressed(Action::Run));
        assert!(action_state.just_pressed(Action::Run));
        assert!(!action_state.released(Action::Run));
        assert!(!action_state.just_released(Action::Run));

        // Waiting
        action_state.tick(Instant::now());
        action_state.update(input_map.which_pressed(&input_streams));

        assert!(action_state.pressed(Action::Run));
        assert!(!action_state.just_pressed(Action::Run));
        assert!(!action_state.released(Action::Run));
        assert!(!action_state.just_released(Action::Run));

        // Releasing
        keyboard_input_stream.release(KeyCode::R);
        let input_streams = InputStreams::from_keyboard(&keyboard_input_stream);

        action_state.update(input_map.which_pressed(&input_streams));
        assert!(!action_state.pressed(Action::Run));
        assert!(!action_state.just_pressed(Action::Run));
        assert!(action_state.released(Action::Run));
        assert!(action_state.just_released(Action::Run));

        // Waiting
        action_state.tick(Instant::now());
        action_state.update(input_map.which_pressed(&input_streams));

        assert!(!action_state.pressed(Action::Run));
        assert!(!action_state.just_pressed(Action::Run));
        assert!(action_state.released(Action::Run));
        assert!(!action_state.just_released(Action::Run));
    }

    #[test]
    fn durations() {
        use bevy::utils::{Duration, Instant};
        use std::thread::sleep;

        let mut action_state = ActionState::<Action>::default();

        // Virtual buttons start released
        assert!(action_state.state(Action::Jump).released());
        assert_eq!(action_state.state(Action::Jump).instant_started(), None,);
        assert_eq!(
            action_state.state(Action::Jump).current_duration(),
            Duration::ZERO
        );
        assert_eq!(
            action_state.state(Action::Jump).previous_duration(),
            Duration::ZERO
        );

        // Pressing a button swaps the state
        action_state.press(Action::Jump);
        assert!(action_state.state(Action::Jump).pressed());
        assert_eq!(action_state.state(Action::Jump).instant_started(), None);
        assert_eq!(
            action_state.state(Action::Jump).current_duration(),
            Duration::ZERO
        );
        assert_eq!(
            action_state.state(Action::Jump).previous_duration(),
            Duration::ZERO
        );

        // Ticking time sets the instant for the new state
        let t0 = Instant::now();
        action_state.tick(t0);
        assert_eq!(action_state.state(Action::Jump).instant_started(), Some(t0));
        assert_eq!(
            action_state.state(Action::Jump).current_duration(),
            Duration::ZERO
        );
        assert_eq!(
            action_state.state(Action::Jump).previous_duration(),
            Duration::ZERO
        );

        // Time passes
        sleep(Duration::from_micros(1));
        let t1 = Instant::now();

        // The duration is updated
        action_state.tick(t1);
        assert_eq!(action_state.state(Action::Jump).instant_started(), Some(t0));
        assert_eq!(action_state.state(Action::Jump).current_duration(), t1 - t0);
        assert_eq!(
            action_state.state(Action::Jump).previous_duration(),
            Duration::ZERO
        );

        // Releasing again, swapping the current duration to the previous one
        action_state.release(Action::Jump);
        assert_eq!(action_state.state(Action::Jump).instant_started(), None);
        assert_eq!(
            action_state.state(Action::Jump).current_duration(),
            Duration::ZERO
        );
        assert_eq!(
            action_state.state(Action::Jump).previous_duration(),
            t1 - t0,
        );
    }
}

/// Stores presses and releases of buttons without timing information
///
/// These are typically accessed using the `Events<ActionDiff>` resource.
/// Uses a minimal storage format, in order to facilitate transport over the network.
///
/// `ID` should be a component type that stores a unique stable identifier for the entity
/// that stores the corresponding [`ActionState`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionDiff<A: Actionlike, ID: Eq + Clone + Component> {
    /// The virtual button was pressed
    Pressed {
        /// The value of the action
        action: A,
        /// The stable identifier of the entity
        id: ID,
    },
    /// The virtual button was released
    Released {
        /// The value of the action
        action: A,
        /// The stable identifier of the entity
        id: ID,
    },
}
