import carpet.api.settings.CarpetRule;
import carpet.api.settings.Validator;
import carpet.settings.ParsedRule;
import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.io.FileWriter;
import java.io.IOException;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Objects;
import java.util.stream.Collectors;
import net.minecraft.text.Text;
import org.apache.commons.lang3.ClassUtils;
import top.hendrixshen.magiclib.carpet.impl.WrappedSettingManager;

public class Printer {
    private static final Gson gson = new Gson();

    public static void print() {
        List<String> ruleNames = new ArrayList<>();
        for (Class<?> clazz : new Class<?>[] {SETTINGS_CLASSES}) {
            for (Field field : clazz.getDeclaredFields()) {
                if (field.getAnnotation(RULE.class) == null) continue;
                ruleNames.add(field.getName());
            }
        }

        JsonArray rules = new JsonArray();
        WrappedSettingManager settingsManager = mixin.PrivateSettingsManagerAccessor.getSettingsManager();
        for (String ruleName : ruleNames) {
            CarpetRule<?> rule = settingsManager.getCarpetRule(ruleName);
            if (rule == null) {
                System.err.println("Warning: rule '" + ruleName + "' could not be found in any SettingsManager");
                continue;
            }

            if (!ruleNames.contains(rule.name())) continue;
            JsonObject obj = new JsonObject();
            obj.addProperty("name", settingsManager.trRuleName(rule.name()));
            obj.addProperty("description", settingsManager.trRuleDesc(rule.name()));
            Class<?> primitive = ClassUtils.wrapperToPrimitive(rule.type());
            obj.addProperty("type", (primitive != null ? primitive : rule.type()).getSimpleName());
            obj.addProperty("value", rule.defaultValue().toString());
            obj.addProperty("strict", rule.strict());
            obj.add(
                    "categories",
                    fromList(rule.categories().stream().map(String::toUpperCase).collect(Collectors.toList())));
            obj.add("options", fromList(rule.suggestions()));
            obj.add(
                    "extras",
                    fromList(settingsManager.trRuleExtraInfo(rule.name()).stream()
                            .map(Text::getString)
                            .collect(Collectors.toList())));
            List<String> validators = new ArrayList<>();
            if (rule instanceof ParsedRule<?> parsedRule) {
                validators.addAll(parsedRule.realValidators.stream()
                        .map(Validator::description)
                        .filter(Objects::nonNull)
                        .toList());
            }
            obj.add("validators", gson.toJsonTree(validators));
            JsonArray configFiles = new JsonArray();
            configFiles.add(settingsManager.getIdentifier());
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

    private static JsonElement fromList(Collection<?> list) {
        if (list.isEmpty()) {
            return new JsonArray();
        }
        return gson.toJsonTree(list);
    }
}
