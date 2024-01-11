from accumulo_access import *;

expr = 'label1&label2'
authorizations = ','.join(['label1'])
result = check_authorization(expr, authorizations)
print("Evaluation of expression: " + expr + " with authorizations: " + authorizations + " is: " + str(result))
authorizations = ','.join(['label1', 'label2'])
result = check_authorization(expr, authorizations)
print("Evaluation of expression: " + expr + " with authorizations: " + authorizations + " is: " + str(result))
