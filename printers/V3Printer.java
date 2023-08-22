import carpet.api.settings.*;
import carpet.settings.ParsedRule;
import carpet.utils.Translations;
import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.io.FileWriter;
import java.io.IOException;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import net.minecraft.text.Text;
import org.apache.commons.lang3.ClassUtils;

public class Printer {
    public static void print() {
        List<String> ruleNames = new ArrayList<>();
        for (Class<?> clazz : new Class<?>[] {SETTINGS_CLASSES}) {
            for (Field field : clazz.getDeclaredFields()) {
                if (field.getAnnotation(RULE.class) == null) continue;
                ruleNames.add(field.getName());
            }
        }

        Gson gson = new Gson();
        JsonArray rules = new JsonArray();
        SettingsManager[] settingsManagers = new SettingsManager[] {SETTINGS_MANAGERS};
        for (String ruleName : ruleNames) {
            CarpetRule<?> rule = null;
            JsonArray configFiles = new JsonArray();
            for (SettingsManager settingsManager : settingsManagers) {
                CarpetRule<?> hasRule = settingsManager.getCarpetRule(ruleName);
                if (hasRule != null) {
                    rule = hasRule;
                    configFiles.add(settingsManager.identifier());
                }
            }
            if (rule == null) {
                System.err.println("Warning: rule '" + ruleName + "' could not be found in any SettingsManager");
                continue;
            }

            if (!ruleNames.contains(rule.name())) continue;
            JsonObject obj = new JsonObject();
            obj.addProperty("name", rule.name());
            obj.addProperty("description", RuleHelper.translatedDescription(rule));
            Class<?> primitive = ClassUtils.wrapperToPrimitive(rule.type());
            obj.addProperty("type", (primitive != null ? primitive : rule.type()).getSimpleName());
            obj.addProperty("value", RuleHelper.toRuleString(rule.defaultValue()));
            obj.addProperty("strict", (rule instanceof ParsedRule<?> pr && pr.isStrict));
            obj.add(
                    "categories",
                    gson.toJsonTree(
                            rule.categories().stream().map(String::toUpperCase).toList()));
            obj.add("options", gson.toJsonTree(rule.suggestions()));
            obj.add(
                    "extras",
                    gson.toJsonTree(
                            rule.extraInfo().stream().map(Text::getString).toList()));
            List<String> validators = new ArrayList<>();
            if (rule instanceof ParsedRule<?> parsedRule) {
                validators.addAll(parsedRule.realValidators.stream()
                        .map(Validator::description)
                        .filter(Objects::nonNull)
                        .toList());
            }
            String additional = Translations.trOrNull(String.format(
                    "%s.rule.%s.additional", rule.settingsManager().identifier(), rule.name()));
            if (additional != null) validators.add(additional);
            obj.add("validators", gson.toJsonTree(validators));
            obj.add("config_files", configFiles);
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
