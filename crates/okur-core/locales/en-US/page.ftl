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
# A region with a name: "Site navigation region".
region-named = { $name } { $region }

# A dialog in a page: its first line, with its name when it has one.
dialog-start = { $name } dialog
dialog-end = dialog end
block-quote = block quote
block-quote-end = block quote end
article = article
article-end = article end
# A named group of controls (a fieldset): its legend follows.
group-start = group start { $name }
group-end = group end { $name }
# The line after a page's menu bar. JAWS writes it as one word.
menu-bar-end = MenuBar end

list-start = list of { $items } items
# A list inside another list: its depth among the lists.
list-start-nested = list of { $items } items nesting level { $level }
list-end = list end
list-end-nested = list end nesting level { $level }
table-end = table end
