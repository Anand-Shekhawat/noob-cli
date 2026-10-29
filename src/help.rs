//! The list shown when you type `help`.

pub const TEXT: &str = "\
Here's everything Noob understands:

  where am i                   show which folder you're in
  show files                   list the files and folders here
  make folder photos           make a new folder (also: make dir photos)
  make file notes.txt          make a new, empty file
  read notes.txt               show what's inside a file
  copy a.txt to b.txt          copy a file (or into a folder: copy a.txt to docs)
  move a.txt into docs         move a file or folder somewhere else
  rename old.txt to new.txt    give a file or folder a new name
  delete notes.txt             delete a file or an empty folder (asks first)
  clear                        clear the screen
  help                         show this list
  bye                          leave Noob (also: exit, quit)

Tip: names can have spaces, like: make folder my photos";
