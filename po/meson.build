i18n.gettext('audiosharegtk', preset: 'glib')

# Name of the POT file to update
pot_file = 'audiosharegtk.pot'

# Temporary POT file in build directory
tmp_pot = 'po/tmp.pot'

# Custom target to update POT
update_pot = custom_target(
  'update-pot',
  input: ['POTFILES.in'],
  output: 'audiosharegtk.pot',
  command: [
    'sh', '-c',
    '''cd @SOURCE_ROOT@ && xgettext --language=Rust --keyword=gettext --from-code=UTF-8 --files-from=po/POTFILES.in --output=po/tmp.pot && xgettext --from-code=UTF-8 --language=Glade --join-existing -o po/tmp.pot src/*.ui && \
    touch po/audiosharegtk.pot && \
    msgmerge --update --backup=none po/audiosharegtk.pot po/tmp.pot && \
    rm -r po/tmp.pot
    '''
  ],
  build_by_default: false
)