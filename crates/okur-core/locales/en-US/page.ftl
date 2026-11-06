## The lines the virtual cursor reads where a page's containers start and end.

# The line after any container's content: "list end", "main region end".
container-end = { $start } end

# Regions (landmarks), as their first line says them.
region-banner = banner
region-complementary = complementary information
region-content-info = content information
region-main = main region
region-navigation = navigation region
region-region = region
region-search = search region
# A form landmark (a named form): "Add Contact Form" [obs, round-aria36, 3061/s1].
region-form = Form
# A region with a name: "Site navigation region".
region-named = { $name } { $region }

# A dialog in a page: its first line, with its name when it has one.
dialog-start = { $name } dialog
dialog-end = dialog end
block-quote = block quote
block-quote-end = block quote end
article = article
article-end = article end
# A named article: "First update article", "First update article end" [obs, 1172/s6, s8].
article-named = { $name } article
article-named-end = { $name } article end
# A feed of articles: "Updates feed", "Updates feed end" [obs, 1172/s5, s12].
feed-start = { $name } feed
feed-end = { $name } feed end
# A note (role note): "note", "note end" [obs, 1172/s16, s18].
note-start = note
note-end = note end
# A toolbar: "Formatting toolbar with 2 controls", "toolbar end" [obs, 1172/s19, s22].
toolbar-start = { $name } toolbar with { $count } controls
toolbar-end = toolbar end
# An editing host (contenteditable) read in the page: its name, then its text between these
# [obs, 1172/s23-s26].
editable-section-start = Editable Section Start
editable-section-end = Editable Section End
# A named group of controls (a fieldset): its legend follows.
group-start = group start { $name }
group-end = group end { $name }
# A page's tab panel, named by its tab: "tab panel start Peter Müller", then "tab panel end".
tab-panel-start = tab panel start { $name }
tab-panel-end = tab panel end
# The lines before and after a page's menu bar's items, the first after its name ("Main menu
# MenuBar with 3 items"). JAWS writes MenuBar as one word.
menu-bar-start = MenuBar with { $items } items
menu-bar-start-named = { $name } MenuBar with { $items } items
menu-bar-end = MenuBar end

list-start = list of { $items } items
# A list inside another list: its depth among the lists.
list-start-nested = list of { $items } items nesting level { $level }
list-end = list end
list-end-nested = list end nesting level { $level }
table-end = table end
grid-end = grid end
