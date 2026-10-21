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
# A menu that opened by itself, not from a menu bar (an application's menu, Alt+F in Chrome).
role-context-menu = Context Menu
role-dialog = dialog
role-edit = Edit
# A text field of several lines, a text document, or a field said before its text.
role-edit-multiline = edit
role-edit-password = password edit
role-edit-read-only = read only edit
# A combo box one types into (an autocomplete).
role-edit-combo = edit combo
# An edit with arrows beside it that step its value (an up-down control).
role-edit-spin-box = edit spin box
# A web page, said after its title.
role-document = document
# The fallback type of an item in the Form Fields list.
role-field = Field
# An image in a page.
role-graphic = Graphic
role-group = group
role-heading-level = heading level { $level }
# A heading's role without its level: JAWS's HeadingIndication=1 [obs, 731/s1].
role-heading = heading
role-link = Link
role-link-same-page = same page link
# The Virtual Viewer's list of a modern application's elements uses lowercase type words.
role-link-viewer = link
role-list = list
role-list-box = List box
# File Explorer's file list. JAWS's spelling.
role-list-box-multi-select = MultiSelect ListBox
role-list-item = list item
role-list-view = List view
role-menu = menu
role-menu-bar = Menu bar
role-menu-item = menu item
# A page of a dialog's tabs (a property sheet's page).
role-page = page
role-progress-bar = progress bar
role-radio-button = radio button
# The regions (landmarks) a focus is in. JAWS writes each as one word.
role-region = Region
role-region-banner = BannerRegion
role-region-complementary = ComplementaryRegion
role-region-content-info = ContentInfoRegion
role-region-form = FormRegion
role-region-main = MainRegion
role-region-navigation = NavigationRegion
role-region-search = SearchRegion
role-slider = slider
# A native slider (a trackbar) that moves left and right.
role-slider-horizontal = left right slider
role-split-button = split button
role-tab = Tab
# A button with a pressed state (aria-pressed).
role-toggle-button = Toggle button
# A tool bar entered by the focus (a Win32 toolbar of buttons).
role-tool-bar = Tool bar
role-tree-item = tree item
role-tree-view = Tree view
role-tree-view-viewer = tree view
# A tree item's depth, said where its role would be.
role-tree-level = level { $level }
# A link that writes an e-mail (mailto:), as JAWS calls it.
role-link-mailto = Send Mail Link
# A horizontal rule in a page.
role-separator = separator
