# Helper to create a path -> data map from a tree's children array
# Preserves the entire object for the path
def build_lookup(children):
  reduce children[] as $item ({};
    . + {($item.path): $item} +  # Store the whole item
    (if $item.type == "directory" and $item.children then build_lookup($item.children) else {} end)
  );

# Helper to update the new tree using the lookup map from the old tree
# It takes an item from the NEW tree and the lookup map from the OLD tree
def update_tree(item; lookup):
  (lookup[item.path] // item) as $old_or_current_item # Use old item if path existed, else use new item
  | item # Start with the structure of the item from the NEW tree
  | .summary = $old_or_current_item.summary # Use summary from old/current item
  # --- Add other fields to preserve from $old_or_current_item here ---
  # | .other_field = $old_or_current_item.other_field
  # ---
  | if item.type == "directory" and item.children then
      # Recurse on children, passing the lookup map down
      .children = (item.children | map(update_tree(.; lookup)))
    else
      . # It's a file, no children to recurse into
    end;

# Main logic
# .[0] is old file content, .[1] is new file content read by --slurp
.[0].children as $old_children
| .[1] as $new_tree # Use the structure of the new tree as the base
| build_lookup($old_children) as $lookup # Build lookup from old tree's children
| $new_tree # Start with the new tree structure
| .children = (.children | map(update_tree(.; $lookup))) # Update children recursively 