## Role words: the type of a control or a container, said after its name.
## JAWS speaks them in a voice of their own. Okur adds the marks around them when it speaks;
## the words here never hold them.
## JAWS's English differs by situation ("Button" on focus, "button" in Say Window), and so do
## the ids: role-<role> is the word focus speech and the virtual cursor use, and a suffix names
## another situation.

role-button = Button
# Said before the text of a virtual cursor line that holds a button, without the role's own voice.
role-button-line = button
role-button-menu = button menu
role-check-box = check box
# Said before the text of a virtual cursor line that holds a check box. JAWS writes it as one word here.
role-check-box-line = checkbox
role-combo-box = combo box
# A combo box taking the focus.
role-combo-box-focus = Combo box
role-dialog = dialog
role-edit = Edit
# A text field of several lines, or a field said before its text.
role-edit-multiline = edit
role-edit-password = password edit
role-edit-read-only = read only edit
# A combo box one types into (an autocomplete).
role-edit-combo = edit combo
# The fallback type of an item in the Form Fields list.
role-field = Field
# A form landmark. JAWS writes it as one word.
role-form-region = FormRegion
role-group = group
role-heading-level = heading level { $level }
role-link = Link
role-link-same-page = same page link
role-list-view = List view
role-progress-bar = progress bar
role-radio-button = radio button
role-slider = slider
role-tab = Tab
# A button with a pressed state (aria-pressed).
role-toggle-button = Toggle button
role-tree-item = tree item
# A tree item's depth, said where its role would be.
role-tree-level = level { $level }
