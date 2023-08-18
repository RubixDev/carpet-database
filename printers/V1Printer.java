import carpet.settings.ParsedRule;
import carpet.settings.Rule;
import carpet.settings.Validator;
import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.io.FileWriter;
import java.io.IOException;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;

public class Printer {
    public static void print() {
        List<String> ruleNames = new ArrayList<>();
        for (Class<?> clazz : new Class<?>[] {SETTINGS_FILES}) {
            for (Field field : clazz.getDeclaredFields()) {
                if (field.getAnnotation(Rule.class) == null) continue;
                ruleNames.add(field.getName());
            }
        }

        Gson gson = new Gson();
        JsonArray rules = new JsonArray();
        for (ParsedRule<?> rule : SETTINGS_MANAGER.getRules()) {
            if (!ruleNames.contains(rule.name)) continue;
            JsonObject obj = new JsonObject();
            obj.addProperty("name", rule.name);
            obj.addProperty("description", rule.description);
            obj.addProperty("type", rule.type.getSimpleName());
            obj.addProperty("value", rule.defaultValue.toString());
            obj.addProperty("strict", rule.isStrict);
            obj.add(
                    "categories",
                    gson.toJsonTree(
                            rule.categories.stream().map(String::toUpperCase).collect(Collectors.toList())));
            obj.add("options", gson.toJsonTree(rule.options));
            obj.add("extras", gson.toJsonTree(rule.extraInfo));
            obj.add(
                    "validators",
                    gson.toJsonTree(rule.validators.stream()
                            .map(Validator::description)
                            .filter(Objects::nonNull)
                            .collect(Collectors.toList())));
            rules.add(obj);
        }

        try (FileWriter writer = new FileWriter("rules.json")) {
            writer.write(gson.toJson(rules));
        } catch (IOException e) {
            throw new RuntimeException(e);
        }

        System.exit(0);
    }
}
