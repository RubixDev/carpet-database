import carpet.api.settings.CarpetRule;
import carpet.api.settings.Rule;
import carpet.api.settings.RuleHelper;
import carpet.api.settings.Validator;
import carpet.settings.ParsedRule;
import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import net.minecraft.text.Text;

import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

public class Printer {
    public static void print() {
        List<String> ruleNames = new ArrayList<>();
        for (Class<?> clazz : new Class<?>[] { SETTINGS_FILES }) {
            for (Field field : clazz.getDeclaredFields()) {
                if (field.getAnnotation(Rule.class) == null) continue;
                ruleNames.add(field.getName());
            }
        }

        Gson gson = new Gson();
        JsonArray rules = new JsonArray();
        for (CarpetRule<?> rule : SETTINGS_MANAGER.getCarpetRules()) {
            if (!ruleNames.contains(rule.name())) continue;
            JsonObject obj = new JsonObject();
            obj.addProperty("name", rule.name());
            obj.addProperty("description", RuleHelper.translatedDescription(rule));
            obj.addProperty("type", rule.type().getSimpleName());
            obj.addProperty("value", RuleHelper.toRuleString(rule.defaultValue()));
            obj.addProperty("strict", (rule instanceof ParsedRule<?> pr && pr.isStrict));
            obj.add("categories", gson.toJsonTree(rule.categories().stream().map(String::toUpperCase).toList()));
            obj.add("options", gson.toJsonTree(rule.suggestions()));
            obj.add("extras", gson.toJsonTree(rule.extraInfo().stream().map(Text::getString).toList()));
            if (rule instanceof ParsedRule<?> parsedRule) {
                obj.add("validators", gson.toJsonTree(parsedRule.realValidators.stream().map(Validator::description).filter(Objects::nonNull).toList()));
            } else {
                obj.add("validators", new JsonArray());
            }
            rules.add(obj);
        }
        System.err.print("");
        System.err.print("|||DATA_START|||");
        System.err.print(gson.toJson(rules));
        System.err.println();
        System.exit(0);
    }
}
