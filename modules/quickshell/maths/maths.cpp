// libqalculate, made once for the whole process and asked by the launcher's
// Calculator and Units on each key (launcher/Numbers.js and Units.js decide
// what's plainly theirs and how it's said). `calculate` gives back, as a map:
//
//   answered  whether it was worked out in time, with no error or warning
//   plain     whether nothing was worked out: a lone number or constant
//   units     whether a unit, or a constant carrying one, is in it
//   currency  whether money is in it, which the Currency answer owns
//   unknown   whether a name it doesn't know, or an unknown such as x, is
//   time      whether a date or time is in it
//   real      whether the result is a real number, which is in `number`
//   complex   whether it's a number with an imaginary part
//   digits    a whole result's exact decimal digits, and its `hex`, `octal`
//             and `binary` ones, each signed and unprefixed
//   text      the result as libqalculate writes it, for what Qt's numbers
//             can't hold
//   amount    a measure's figure, and `unit` its unit, as `m/s`
//   fits      whether a measure is in the unit asked for after "to"
#include "maths.h"

#include <clocale>
#include <future>
#include <libqalculate/qalculate.h>
#include <string>

namespace {

// The most milliseconds one expression may take, well inside a key's
// budget, past which it simply isn't answered.
constexpr int patience = 50;
// The longest whole number given digit by digit; longer ones are written
// with an exponent, as no one reads past this.
constexpr size_t mostDigits = 64;

// Functions that reach outside the calculation, which a key must never set
// off: running a program, drawing a plot with gnuplot, reading or writing a
// file, and keeping a variable or a stack that a later key would find.
constexpr const char *outsideFunctions[] = {
    "command", "plot", "load", "export", "save", "register", "stack"};

// libqalculate keeps one calculator for the whole process, so it's made
// once however many engines the shell makes as it reloads. Its definitions
// are read on a thread of their own, a tenth of a second the shell needn't
// wait for, and used only on the shell's thread once they're read.
class Engine {
public:
    Engine() {
        // libqalculate told to ignore the locale sets the whole process to
        // C as it's made, which would change Qt's own text handling, so the
        // locale is put back, all but numbers, which libqalculate reads and
        // writes with a point and nothing in Qt reads.
        const std::string before = std::setlocale(LC_ALL, nullptr);
        new Calculator(true);
        std::setlocale(LC_ALL, before.c_str());
        std::setlocale(LC_NUMERIC, "C");
        loading = std::async(std::launch::async,
                             [] { return CALCULATOR->loadGlobalDefinitions(); });
    }

    bool ready() {
        if (!loaded) {
            usable = loading.get();
            loaded = true;
            settle();
        }
        return usable;
    }

    EvaluationOptions evaluation;
    // How a number is written: in full, with a point and no grouping.
    PrintOptions numbers;
    // How a unit is written: its symbol, with its own signs, as m² or °C.
    PrintOptions units;

private:
    // Each setting made on the shell's thread, once the definitions are in.
    void settle() {
        for (const char *name : outsideFunctions)
            if (MathFunction *function = CALCULATOR->getActiveFunction(name))
                function->setActive(false);
        CALCULATOR->setPrecision(16);
        CALCULATOR->useIntervalArithmetic(false);
        evaluation.approximation = APPROXIMATION_APPROXIMATE;
        evaluation.parse_options.angle_unit = ANGLE_UNIT_RADIANS;
        evaluation.mixed_units_conversion = MIXED_UNITS_CONVERSION_NONE;
        numbers.min_exp = EXP_NONE;
        numbers.number_fraction_format = FRACTION_DECIMAL;
        numbers.base_display = BASE_DISPLAY_NONE;
        numbers.lower_case_numbers = true;
        numbers.interval_display = INTERVAL_DISPLAY_MIDPOINT;
        numbers.twos_complement = false;
        numbers.hexadecimal_twos_complement = false;
        units.abbreviate_names = true;
        units.use_unicode_signs = true;
        units.negative_exponents = false;
        units.division_sign = DIVISION_SIGN_SLASH;
        units.spacious = false;
        units.interval_display = INTERVAL_DISPLAY_MIDPOINT;
    }

    std::future<bool> loading;
    bool loaded = false;
    bool usable = false;
};

Engine &engine() {
    static Engine made;
    return made;
}

bool hasCurrency(const MathStructure &structure) {
    if (structure.isUnit() && structure.unit()->isCurrency())
        return true;
    for (size_t at = 0; at < structure.size(); at++)
        if (hasCurrency(structure[at]))
            return true;
    return false;
}

// Whether a structure is units alone, multiplied and raised to powers, as
// what's left of a measure once its figure is taken out.
bool onlyUnits(const MathStructure &structure) {
    if (structure.isUnit_exp())
        return true;
    if (!structure.isMultiplication() || structure.size() == 0)
        return false;
    for (size_t at = 0; at < structure.size(); at++)
        if (!structure[at].isUnit_exp())
            return false;
    return true;
}

// Whether a result is a measure: one figure times units, as a sum of two
// units isn't.
bool isMeasure(const MathStructure &measure) {
    MathStructure unit(measure);
    if (unit.isMultiplication() && unit.size() > 1 && unit[0].isNumber()) {
        if (!unit[0].number().isReal())
            return false;
        unit.delChild(1, true);
    }
    return onlyUnits(unit);
}

// A measure as libqalculate writes it, with the prefix it picks, as 100
// km/h rather than 100,000 m/h: its figure, and its unit's text.
void measured(MathStructure measure, const PrintOptions &options, Number &amount,
              QString &unit) {
    measure.format(options);
    amount.set(1, 1);
    const bool negative = measure.isNegate();
    if (negative)
        measure.setToChild(1);
    // A rate is written as a division, its figure in the part on top.
    MathStructure &top = measure.isDivision() ? measure[0] : measure;
    if (top.isMultiplication() && top.size() > 1 && top[0].isNumber()) {
        amount = top[0].number();
        top.delChild(1, true);
    } else if (top.isNumber()) {
        amount = top.number();
        top.set(1, 1);
    }
    if (negative)
        amount.negate();
    unit = QString::fromStdString(measure.print(options));
}

// Whether a reading did nothing: a lone number or constant, signed or not.
bool isPlain(const MathStructure &parsed) {
    const MathStructure &inner = parsed.isNegate() ? parsed[0] : parsed;
    return inner.isNumber() || inner.isVariable();
}

QString written(MathStructure structure, const PrintOptions &options) {
    structure.format(options);
    return QString::fromStdString(structure.print(options));
}

QString inBase(const Number &number, int base) {
    PrintOptions options = engine().numbers;
    options.base = base;
    return QString::fromStdString(number.print(options));
}

} // namespace

Maths::Maths(QObject *parent) : QObject(parent) {
    engine();
}

void Maths::prepare() const {
    engine();
}

QVariantMap Maths::calculate(const QString &expression) const {
    if (expression == lastAsked && !lastSaid.isEmpty())
        return lastSaid;
    QVariantMap said{{"answered", false}};
    Engine &maths = engine();
    if (!maths.ready())
        return said;
    CALCULATOR->clearMessages();
    MathStructure parsed;
    MathStructure target;
    CALCULATOR->startControl(patience);
    const MathStructure result = CALCULATOR->calculate(
        CALCULATOR->unlocalizeExpression(expression.toStdString(),
                                         maths.evaluation.parse_options),
        maths.evaluation, &parsed, &target);
    // A warning is as good as a refusal: libqalculate warns where it
    // guessed, as at "12*" or "8.8.8.8", and those aren't sums.
    bool troubled = CALCULATOR->aborted() || result.isAborted();
    for (CalculatorMessage *message = CALCULATOR->message(); message != nullptr;
         message = CALCULATOR->nextMessage())
        troubled = troubled || message->type() != MESSAGE_INFORMATION;
    if (!troubled) {
        said["answered"] = true;
        said["plain"] = isPlain(parsed);
        said["units"] = parsed.containsType(STRUCT_UNIT, false, true, true) > 0
                        || result.containsType(STRUCT_UNIT, false, true, true) > 0;
        said["currency"] = hasCurrency(parsed) || hasCurrency(result);
        said["unknown"] = parsed.containsUnknowns() || result.containsUnknowns();
        said["time"] = parsed.containsType(STRUCT_DATETIME, false, true, true) > 0
                       || result.containsType(STRUCT_DATETIME, false, true, true) > 0;
        const bool number = result.isNumber();
        const bool real = number && result.number().isReal();
        said["real"] = real;
        said["complex"] = number && !real;
        if (real)
            said["number"] = result.number().floatValue();
        if (real && result.number().isInteger()) {
            const QString digits = inBase(result.number(), BASE_DECIMAL);
            if (static_cast<size_t>(digits.size()) <= mostDigits) {
                said["digits"] = digits;
                said["hex"] = inBase(result.number(), BASE_HEXADECIMAL);
                said["octal"] = inBase(result.number(), BASE_OCTAL);
                said["binary"] = inBase(result.number(), BASE_BINARY);
            }
        }
        PrintOptions plainly = maths.numbers;
        plainly.min_exp = EXP_PRECISION;
        said["text"] = written(result, plainly);
        if (isMeasure(result)) {
            Number amount;
            QString unit;
            measured(result, maths.units, amount, unit);
            said["amount"] = amount.floatValue();
            said["unit"] = unit;
            // What was asked for after "to", written the same way, so a
            // measure that couldn't be put in it, as 5 km in kg, says so.
            Number wanted;
            QString asked;
            if (!target.isUndefined())
                measured(target, maths.units, wanted, asked);
            said["fits"] = !target.isUndefined() && asked == unit;
        }
    }
    CALCULATOR->stopControl();
    lastAsked = expression;
    lastSaid = said;
    return said;
}
