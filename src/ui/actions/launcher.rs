use gpui::actions;

actions!(
    example_input,
    [
        Quit,
        ItemDown,
        ItemUp,
        ItemLeft,
        ItemRight,
        NextVar,
        PrevVar,
        Execute,
        ExecuteInplace,
        OpenContext,
        /// Go back: dispatched by an empty search bar's backspace.
        Back,
    ]
);
