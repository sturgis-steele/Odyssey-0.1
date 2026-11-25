"""A bounded arithmetic grammar evaluated by walking Python's expression AST."""
import ast
import math
import operator

from pydantic import Field

from ..contracts import Input


class Calculation(Input):
    expression: str = Field(min_length=1, max_length=192, description="Arithmetic, parentheses, constants pi and e, or a function call like sqrt(x), sin(x), log(x) or exp(x).")


BINARY = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul,
          ast.Div: operator.truediv, ast.FloorDiv: operator.floordiv, ast.Mod: operator.mod,
          ast.Pow: operator.pow}
FUNCTIONS = {"abs": abs, "acos": math.acos, "acosh": math.acosh, "asin": math.asin,
             "asinh": math.asinh, "atan": math.atan, "atanh": math.atanh, "ceil": math.ceil,
             "cos": math.cos, "cosh": math.cosh, "exp": math.exp, "floor": math.floor,
             "ln": math.log, "log": math.log, "log2": math.log2, "log10": math.log10,
             "sin": math.sin, "sinh": math.sinh, "sqrt": math.sqrt, "tan": math.tan,
             "tanh": math.tanh}
CONSTANTS = {"pi": math.pi, "e": math.e}
LIMIT = 10**100


def evaluate(node: ast.AST) -> int | float:
    if isinstance(node, ast.Constant) and type(node.value) in (int, float):
        value = node.value
    elif isinstance(node, ast.Name) and node.id in CONSTANTS:
        value = CONSTANTS[node.id]
    elif isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.USub, ast.UAdd)):
        operand = evaluate(node.operand)
        value = -operand if isinstance(node.op, ast.USub) else operand
    elif isinstance(node, ast.BinOp) and type(node.op) in BINARY:
        left, right = evaluate(node.left), evaluate(node.right)
        if isinstance(node.op, ast.Pow) and abs(right) > 100:
            raise ValueError("Use an exponent between -100 and 100.")
        value = BINARY[type(node.op)](left, right)
    elif (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id in FUNCTIONS
          and len(node.args) == 1 and not node.keywords):
        try:
            value = FUNCTIONS[node.func.id](evaluate(node.args[0]))
        except OverflowError:
            raise ValueError("The result exceeds the supported real-number range.")
    else:
        raise ValueError("Use numbers, arithmetic operators, parentheses or a function call like sqrt(x) or sin(x).")
    if type(value) not in (int, float) or abs(value) > LIMIT or not math.isfinite(value):
        raise ValueError("The result exceeds the supported real-number range.")
    return value


def calculate(args: Calculation) -> dict:
    try:
        root = ast.parse(args.expression.strip(), mode="eval")
        if sum(1 for _ in ast.walk(root)) > 64:
            raise ValueError("Use a shorter expression.")
        return {"value": evaluate(root.body)}
    except (SyntaxError, ArithmeticError, RecursionError) as exc:
        raise ValueError("This expression cannot be calculated. Check its syntax and operands.") from exc
