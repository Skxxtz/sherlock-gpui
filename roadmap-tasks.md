# Introduction 
> [!WARNING]
> Read this before you choose a task / topic.

This file contains some tasks that came to mind for porting Sherlock from GTK4
to GPUI. It is by far not a complete collection of everything though! I will
mark what people are working on with a [in progress] tag. That way, there wont
be dupplicate efforts on the same problem. Additionally, there will be a
[depends on] tag. This indicates that some tasks should not be touched if
another one is currently being worked on. For example, if a file should be
refactored, it would cause merge conflicts if someone else would add code to
it.


# Tasks

## [1] Improve Launcher Loading from `fallback.json`
[in progress (pretty much done for now)] 
This step includes several sub tasks:
- [x] Implement launcher loader 
- [ ] Improve the launcher logic (maybe macro for improved extesibility)
- [ ] Add bincode cache for the launcher logic with a trigger on original file change


## [2] Clean up `search_view.rs` 
[done for now]
The file is currently over 700 lines long. It should be refactored before
anything is added to it.

## [3] Add Filter/Sorter to `search_view`
[done for now]
Currently, the search bar takes inputs, but the input does not change the
sorting and filtering of the content.

