# C-036: File-list navigation must preserve browsing position

Status: active

File-list navigation must persist and restore scroll position by browsing scope:
folder, sort, MIME filter, and search text. This applies to browser Back,
folder entry, route remounts, and page refresh.

Rules:

- Do not hardcode Settings Back or similar page-return actions to `/files` when
  browser history can preserve the previous route and query params.
- Do not force file-list scroll to top after every folder navigation.
- Scroll to top only when the destination scope has no saved browsing position.
- Persist the active file-list position during normal scrolling and before
  pagehide/visibility cleanup, not only when navigating to another folder.
- Add regression tests before changing navigation or scroll restoration logic.

Reason: hardcoded returns and unconditional top-scrolls drop folder context and
make users lose their place after entering folders, pressing Back, or refreshing.
