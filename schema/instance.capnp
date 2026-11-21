# DRAFT contract for the single instance of a program window (fs-starter, ADR 0097, step e2-host-pflicht). Owner: fs-starter (host).
# The first window of a program serves `Instance` on the socket `<runtime dir>/<program>.window.sock`; a second start of the same
# program does not open a second window: it connects, calls `open` with its words and ends. Nothing here executes the words:
# the serving side parses them with its normal word reader and answers `accepted = false` for words it does not know.
# Sizes are limited by the fs-rpc read limits. Evolution (Cap'n Proto): only append, never renumber or remove.

@0xa36ccf61f34f0d59;

interface Instance {
  # The words a second start was given (after the program name), handed to the running window.
  # accepted = true: the running window took them; false: refused (unknown words), nothing was done.
  open @0 (words :List(Text)) -> (accepted :Bool);
}
