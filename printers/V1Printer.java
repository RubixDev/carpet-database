import carpet.settings.ParsedRule;
import carpet.settings.Validator;
import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;

public class Printer {
    public static void print() {
        Gson gson = new Gson();
        JsonArray rules = new JsonArray();
        for (ParsedRule<?> rule : SETTINGS_MANAGER.getRules()) {
            JsonObject obj = new JsonObject();
            obj.addProperty("name", rule.name);
            obj.addProperty("description", rule.description);
            obj.addProperty("type", rule.type.getSimpleName());
            obj.addProperty("value", rule.defaultValue.toString());
            obj.addProperty("strict", rule.isStrict);
            obj.add("categories", gson.toJsonTree(rule.categories.stream().map(String::toUpperCase).collect(Collectors.toList())));
            obj.add("options", gson.toJsonTree(rule.options));
            obj.add("extras", gson.toJsonTree(rule.extraInfo));
            obj.add("validators", gson.toJsonTree(
                    rule.validators.stream().map(Validator::description).filter(Objects::nonNull).collect(Collectors.toList())));
            rules.add(obj);
        }
        System.err.print("");
        System.err.print("|||DATA_START|||");
        System.err.print(gson.toJson(rules));
        System.err.println();
        System.exit(0);
    }
}
