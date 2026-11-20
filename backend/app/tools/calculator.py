"""A bounded arithmetic grammar evaluated by walking Python's expression AST."""
import ast
import math
import operator

from pydantic import Field

from ..contracts import Input


class Calculation(Input):
    expression: str = Field(min_length=1, max_length=192, description="Arithmetic, parentheses or sqrt(x).")


BINARY = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul,
          ast.Div: operator.truediv, ast.FloorDiv: operator.floordiv, ast.Mod: operator.mod,
          ast.Pow: operator.pow}
LIMIT = 10**100


def evaluate(node: ast.AST) -> int | float:
    if isinstance(node, ast.Constant) and type(node.value) in (int, float):
        value = node.value
    elif isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.USub, ast.UAdd)):
        operand = evaluate(node.operand)
        value = -operand if isinstance(node.op, ast.USub) else operand
    elif isinstance(node, ast.BinOp) and type(node.op) in BINARY:
        left, right = evaluate(node.left), evaluate(node.right)
        if isinstance(node.op, ast.Pow) and abs(right) > 100:
            raise ValueError("Use an exponent between -100 and 100.")
        value = BINARY[type(node.op)](left, right)
    elif (isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "sqrt"
          and len(node.args) == 1 and not node.keywords):
        value = math.sqrt(evaluate(node.args[0]))
    else:
        raise ValueError("Use numbers, arithmetic operators, parentheses or sqrt(x).")
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
